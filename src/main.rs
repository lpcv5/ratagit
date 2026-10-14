mod app;
mod config;
mod file_tree;
mod flux;
mod git;
mod ui;

use app::App;
use color_eyre::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
};
use flux::action::{Action, SystemAction};
use flux::runtime::EffectExecutor;
use flux::snapshot::AppStateSnapshotOwned;
use ratatui::DefaultTerminal;
use std::fs::{File, OpenOptions};
use std::io;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;
use tokio::sync::{broadcast, mpsc, watch};
use tracing::{error, info};

const ENABLE_DEBUG_LOG: bool = cfg!(debug_assertions);
const APP_LOG_FILE: &str = "ratagit.log";

fn init_logging(debug_mode: bool) {
    let max_level = if debug_mode {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };
    // TUI should never print tracing logs to terminal.
    if let Ok(file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(APP_LOG_FILE)
    {
        let _ = tracing_subscriber::fmt()
            .with_max_level(max_level)
            .with_ansi(false)
            .without_time()
            .with_target(false)
            .with_level(false)
            .with_writer(file)
            .try_init();
    } else {
        let _ = tracing_subscriber::fmt()
            .with_max_level(max_level)
            .with_ansi(false)
            .without_time()
            .with_target(false)
            .with_level(false)
            .with_writer(io::sink)
            .try_init();
    }
}

#[derive(Default)]
struct PerfCounters {
    ui_events: AtomicU64,
    ui_messages: AtomicU64,
    ui_ticks_sent: AtomicU64,
    ui_draws: AtomicU64,
    ui_draw_us: AtomicU64,
    dispatch_actions: AtomicU64,
    dispatch_commands: AtomicU64,
    dispatch_us: AtomicU64,
    dispatch_lock_wait_us: AtomicU64,
    dispatch_reduce_us: AtomicU64,
    effect_commands: AtomicU64,
    effect_actions_emitted: AtomicU64,
    effect_us: AtomicU64,
    action_enqueued: AtomicU64,
    action_dequeued: AtomicU64,
    action_backlog_max: AtomicU64,
    command_enqueued: AtomicU64,
    command_dequeued: AtomicU64,
    command_backlog_max: AtomicU64,
    ui_lock_wait_us: AtomicU64,
    ui_key_handle_us: AtomicU64,
    ui_key_handle_max_us: AtomicU64,
    ui_draw_max_us: AtomicU64,
    dispatch_max_us: AtomicU64,
}

#[derive(Default, Clone, Copy)]
struct PerfSnapshot {
    ui_events: u64,
    ui_messages: u64,
    ui_ticks_sent: u64,
    ui_draws: u64,
    ui_draw_us: u64,
    dispatch_actions: u64,
    dispatch_commands: u64,
    dispatch_us: u64,
    dispatch_lock_wait_us: u64,
    dispatch_reduce_us: u64,
    effect_commands: u64,
    effect_actions_emitted: u64,
    effect_us: u64,
    action_enqueued: u64,
    action_dequeued: u64,
    action_backlog_max: u64,
    command_enqueued: u64,
    command_dequeued: u64,
    command_backlog_max: u64,
    ui_lock_wait_us: u64,
    ui_key_handle_us: u64,
    ui_key_handle_max_us: u64,
    ui_draw_max_us: u64,
    dispatch_max_us: u64,
}

impl PerfCounters {
    fn snapshot(&self) -> PerfSnapshot {
        PerfSnapshot {
            ui_events: self.ui_events.load(Ordering::Relaxed),
            ui_messages: self.ui_messages.load(Ordering::Relaxed),
            ui_ticks_sent: self.ui_ticks_sent.load(Ordering::Relaxed),
            ui_draws: self.ui_draws.load(Ordering::Relaxed),
            ui_draw_us: self.ui_draw_us.load(Ordering::Relaxed),
            dispatch_actions: self.dispatch_actions.load(Ordering::Relaxed),
            dispatch_commands: self.dispatch_commands.load(Ordering::Relaxed),
            dispatch_us: self.dispatch_us.load(Ordering::Relaxed),
            dispatch_lock_wait_us: self.dispatch_lock_wait_us.load(Ordering::Relaxed),
            dispatch_reduce_us: self.dispatch_reduce_us.load(Ordering::Relaxed),
            effect_commands: self.effect_commands.load(Ordering::Relaxed),
            effect_actions_emitted: self.effect_actions_emitted.load(Ordering::Relaxed),
            effect_us: self.effect_us.load(Ordering::Relaxed),
            action_enqueued: self.action_enqueued.load(Ordering::Relaxed),
            action_dequeued: self.action_dequeued.load(Ordering::Relaxed),
            action_backlog_max: self.action_backlog_max.load(Ordering::Relaxed),
            command_enqueued: self.command_enqueued.load(Ordering::Relaxed),
            command_dequeued: self.command_dequeued.load(Ordering::Relaxed),
            command_backlog_max: self.command_backlog_max.load(Ordering::Relaxed),
            ui_lock_wait_us: self.ui_lock_wait_us.load(Ordering::Relaxed),
            ui_key_handle_us: self.ui_key_handle_us.load(Ordering::Relaxed),
            ui_key_handle_max_us: self.ui_key_handle_max_us.load(Ordering::Relaxed),
            ui_draw_max_us: self.ui_draw_max_us.load(Ordering::Relaxed),
            dispatch_max_us: self.dispatch_max_us.load(Ordering::Relaxed),
        }
    }
}

fn update_max(metric: &AtomicU64, value: u64) {
    let mut prev = metric.load(Ordering::Relaxed);
    while value > prev {
        match metric.compare_exchange_weak(prev, value, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(actual) => prev = actual,
        }
    }
}

fn update_backlog(enqueued: &AtomicU64, dequeued: &AtomicU64, backlog_max: &AtomicU64) {
    let enq = enqueued.load(Ordering::Relaxed);
    let deq = dequeued.load(Ordering::Relaxed);
    let backlog = enq.saturating_sub(deq);
    update_max(backlog_max, backlog);
}

struct PerfLog {
    file: File,
    last_flush: Instant,
    last_snapshot: PerfSnapshot,
}

impl PerfLog {
    fn new() -> io::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open("ratagit-debug.log")?;
        Ok(Self {
            file,
            last_flush: Instant::now(),
            last_snapshot: PerfSnapshot::default(),
        })
    }

    fn write_line(&mut self, line: &str) {
        use std::io::Write;
        let _ = writeln!(self.file, "{}", line);
        let _ = self.file.flush();
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let debug_mode = std::env::args().any(|a| a == "--debug");
    init_logging(debug_mode);
    info!("app_start debug={}", debug_mode);

    if debug_mode {
        git::enable_git_job_log("ratagit-git-jobs.log");
    }

    let app = App::new()?;
    let mut terminal = ratatui::try_init().inspect_err(|_| ratatui::restore())?;
    // Ratatui restores raw mode and the alternate screen before color-eyre's hook.
    // Mouse capture is application-specific and must also end before diagnostics.
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = execute!(io::stdout(), DisableMouseCapture);
        hook(info);
    }));
    let res = async {
        execute!(io::stdout(), EnableMouseCapture)?;
        run_app(&mut terminal, app).await
    }
    .await;

    // Attempt both cleanup steps, even if one fails. Preserve the app's error.
    let mouse_result = execute!(io::stdout(), DisableMouseCapture);
    let restore_result = ratatui::try_restore();
    drop(terminal);

    if let Err(err) = &res {
        error!("app_error err={:?}", err);
    }

    info!(
        "app_exit ok={}",
        res.is_ok() && mouse_result.is_ok() && restore_result.is_ok()
    );
    res?;
    mouse_result?;
    restore_result?;
    Ok(())
}

async fn run_app(terminal: &mut DefaultTerminal, app: App) -> Result<()> {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async move {
            let App { state, runtime } = app;
            let perf = Arc::new(PerfCounters::default());
            let (action_tx, action_rx) = mpsc::unbounded_channel::<Action>();
            let (command_tx, command_rx) = mpsc::unbounded_channel::<app::Command>();
            let (state_version_tx, mut state_version_rx) = watch::channel(0u64);
            let initial_snapshot = Arc::new(AppStateSnapshotOwned::from_app(&state));
            let (snapshot_tx, snapshot_rx) = watch::channel(initial_snapshot);
            let (shutdown_tx, _) = broadcast::channel::<()>(1);

            let dispatch_handle = tokio::task::spawn_local(dispatch_loop(
                state,
                perf.clone(),
                action_rx,
                command_tx,
                state_version_tx,
                snapshot_tx,
                shutdown_tx.subscribe(),
            ));
            let effect_handle = tokio::task::spawn_local(effect_loop(
                runtime,
                perf.clone(),
                command_rx,
                action_tx.clone(),
                shutdown_tx.subscribe(),
            ));
            let auto_refresh_handle = tokio::task::spawn_local(auto_refresh_loop(
                action_tx.clone(),
                shutdown_tx.subscribe(),
            ));

            let ui_result = ui_loop(
                terminal,
                perf,
                action_tx,
                &mut state_version_rx,
                snapshot_rx,
                shutdown_tx.clone(),
            )
            .await;

            let _ = shutdown_tx.send(());
            let _ = dispatch_handle.await;
            let _ = effect_handle.await;
            let _ = auto_refresh_handle.await;
            ui_result
        })
        .await
}

async fn ui_loop(
    terminal: &mut DefaultTerminal,
    perf: Arc<PerfCounters>,
    action_tx: mpsc::UnboundedSender<Action>,
    state_version_rx: &mut watch::Receiver<u64>,
    mut snapshot_rx: watch::Receiver<Arc<AppStateSnapshotOwned>>,
    shutdown_tx: broadcast::Sender<()>,
) -> Result<()> {
    const MAX_EVENTS_PER_FRAME: usize = 64;
    const DIFF_RELOAD_DEBOUNCE: Duration = Duration::from_millis(180);
    const SLOW_DRAW_US: u64 = 8_000;
    let mut perf_log = if ENABLE_DEBUG_LOG {
        PerfLog::new().ok()
    } else {
        None
    };
    if let Some(log) = perf_log.as_mut() {
        log.write_line("ratagit debug perf started");
    }

    loop {
        // Check exit status at loop start
        {
            let snapshot = snapshot_rx.borrow();
            if !snapshot.running {
                info!("ui_loop: detected running=false, exiting");
                let _ = shutdown_tx.send(());
                return Ok(());
            }
        }

        let mut processed_input_event = false;
        if state_version_rx.has_changed().unwrap_or(false) {
            let _ = state_version_rx.borrow_and_update();
        }

        if snapshot_rx.has_changed().unwrap_or(false) {
            let draw_started = Instant::now();
            terminal.draw(|f| {
                let snapshot = snapshot_rx.borrow_and_update();
                let view = snapshot.as_snapshot();
                crate::ui::layout::render_layout(f, &view);
            })?;
            let draw_us = draw_started.elapsed().as_micros() as u64;
            perf.ui_draws.fetch_add(1, Ordering::Relaxed);
            perf.ui_draw_us.fetch_add(draw_us, Ordering::Relaxed);
            update_max(&perf.ui_draw_max_us, draw_us);
            if let Some(log) = perf_log.as_mut() {
                if draw_us >= SLOW_DRAW_US {
                    log.write_line(&format!("slow_draw_us={}", draw_us));
                }
            }
        }

        if event::poll(Duration::from_millis(0))? {
            for _ in 0..MAX_EVENTS_PER_FRAME {
                match event::read()? {
                    Event::Key(key) => {
                        processed_input_event = true;
                        perf.ui_events.fetch_add(1, Ordering::Relaxed);
                        if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
                            let key_started = Instant::now();
                            let snapshot = snapshot_rx.borrow();
                            let view = snapshot.as_snapshot();
                            let mapped_actions = flux::input_mapper::map_key_to_actions(key, &view);
                            for action in mapped_actions {
                                let _ = action_tx.send(action);
                                perf.ui_messages.fetch_add(1, Ordering::Relaxed);
                                perf.action_enqueued.fetch_add(1, Ordering::Relaxed);
                                update_backlog(
                                    &perf.action_enqueued,
                                    &perf.action_dequeued,
                                    &perf.action_backlog_max,
                                );
                            }
                            let key_us = key_started.elapsed().as_micros() as u64;
                            perf.ui_key_handle_us.fetch_add(key_us, Ordering::Relaxed);
                            update_max(&perf.ui_key_handle_max_us, key_us);
                        }
                    }
                    Event::Resize(width, height) => {
                        processed_input_event = true;
                        perf.ui_events.fetch_add(1, Ordering::Relaxed);
                        let _ =
                            action_tx.send(Action::System(SystemAction::Resize { width, height }));
                        perf.action_enqueued.fetch_add(1, Ordering::Relaxed);
                        update_backlog(
                            &perf.action_enqueued,
                            &perf.action_dequeued,
                            &perf.action_backlog_max,
                        );
                    }
                    _ => {}
                }

                if !event::poll(Duration::from_millis(0))? {
                    break;
                }
            }
        }

        let should_tick = {
            let snapshot = snapshot_rx.borrow();
            snapshot.should_tick(DIFF_RELOAD_DEBOUNCE)
        };
        if should_tick {
            perf.ui_ticks_sent.fetch_add(1, Ordering::Relaxed);
            let _ = action_tx.send(Action::System(SystemAction::Tick));
            perf.action_enqueued.fetch_add(1, Ordering::Relaxed);
            update_backlog(
                &perf.action_enqueued,
                &perf.action_dequeued,
                &perf.action_backlog_max,
            );
        }

        if let Some(log) = perf_log.as_mut() {
            if log.last_flush.elapsed() >= Duration::from_secs(1) {
                let now = perf.snapshot();
                let prev = log.last_snapshot;
                let delta = PerfSnapshot {
                    ui_events: now.ui_events.saturating_sub(prev.ui_events),
                    ui_messages: now.ui_messages.saturating_sub(prev.ui_messages),
                    ui_ticks_sent: now.ui_ticks_sent.saturating_sub(prev.ui_ticks_sent),
                    ui_draws: now.ui_draws.saturating_sub(prev.ui_draws),
                    ui_draw_us: now.ui_draw_us.saturating_sub(prev.ui_draw_us),
                    dispatch_actions: now.dispatch_actions.saturating_sub(prev.dispatch_actions),
                    dispatch_commands: now.dispatch_commands.saturating_sub(prev.dispatch_commands),
                    dispatch_us: now.dispatch_us.saturating_sub(prev.dispatch_us),
                    dispatch_lock_wait_us: now
                        .dispatch_lock_wait_us
                        .saturating_sub(prev.dispatch_lock_wait_us),
                    dispatch_reduce_us: now
                        .dispatch_reduce_us
                        .saturating_sub(prev.dispatch_reduce_us),
                    effect_commands: now.effect_commands.saturating_sub(prev.effect_commands),
                    effect_actions_emitted: now
                        .effect_actions_emitted
                        .saturating_sub(prev.effect_actions_emitted),
                    effect_us: now.effect_us.saturating_sub(prev.effect_us),
                    action_enqueued: now.action_enqueued.saturating_sub(prev.action_enqueued),
                    action_dequeued: now.action_dequeued.saturating_sub(prev.action_dequeued),
                    action_backlog_max: now.action_backlog_max,
                    command_enqueued: now.command_enqueued.saturating_sub(prev.command_enqueued),
                    command_dequeued: now.command_dequeued.saturating_sub(prev.command_dequeued),
                    command_backlog_max: now.command_backlog_max,
                    ui_lock_wait_us: now.ui_lock_wait_us.saturating_sub(prev.ui_lock_wait_us),
                    ui_key_handle_us: now.ui_key_handle_us.saturating_sub(prev.ui_key_handle_us),
                    ui_key_handle_max_us: now.ui_key_handle_max_us,
                    ui_draw_max_us: now.ui_draw_max_us,
                    dispatch_max_us: now.dispatch_max_us,
                };
                let avg_draw_us = delta.ui_draw_us.checked_div(delta.ui_draws).unwrap_or(0);
                let avg_ui_lock_wait_us = delta
                    .ui_lock_wait_us
                    .checked_div(delta.ui_events)
                    .unwrap_or(0);
                let avg_ui_key_us = delta
                    .ui_key_handle_us
                    .checked_div(delta.ui_messages)
                    .unwrap_or(0);
                let avg_dispatch_us = delta
                    .dispatch_us
                    .checked_div(delta.dispatch_actions)
                    .unwrap_or(0);
                let avg_dispatch_lock_wait_us = delta
                    .dispatch_lock_wait_us
                    .checked_div(delta.dispatch_actions)
                    .unwrap_or(0);
                let avg_dispatch_reduce_us = delta
                    .dispatch_reduce_us
                    .checked_div(delta.dispatch_actions)
                    .unwrap_or(0);
                let avg_effect_us = delta
                    .effect_us
                    .checked_div(delta.effect_commands)
                    .unwrap_or(0);
                let pending_tasks = 0usize;
                let task_metrics = crate::flux::task_manager::TaskMetrics::default();
                log.write_line(&format!(
                    "1s ui_events={} ui_messages={} ticks={} draws={} avg_draw_us={} max_draw_us={} avg_ui_lock_wait_us={} avg_ui_key_us={} max_ui_key_us={} dispatch_actions={} avg_dispatch_us={} max_dispatch_us={} avg_dispatch_lock_wait_us={} avg_dispatch_reduce_us={} dispatch_commands={} effect_commands={} avg_effect_us={} effect_actions={} action_backlog={} action_backlog_max={} command_backlog={} command_backlog_max={} pending_tasks={} task_enqueued={} task_dequeued={} task_ready={} task_finished={} task_failed={} task_cancelled={} task_stale_dropped={} task_queue_dropped={}",
                    delta.ui_events,
                    delta.ui_messages,
                    delta.ui_ticks_sent,
                    delta.ui_draws,
                    avg_draw_us,
                    delta.ui_draw_max_us,
                    avg_ui_lock_wait_us,
                    avg_ui_key_us,
                    delta.ui_key_handle_max_us,
                    delta.dispatch_actions,
                    avg_dispatch_us,
                    delta.dispatch_max_us,
                    avg_dispatch_lock_wait_us,
                    avg_dispatch_reduce_us,
                    delta.dispatch_commands,
                    delta.effect_commands,
                    avg_effect_us,
                    delta.effect_actions_emitted,
                    now.action_enqueued.saturating_sub(now.action_dequeued),
                    delta.action_backlog_max,
                    now.command_enqueued.saturating_sub(now.command_dequeued),
                    delta.command_backlog_max,
                    pending_tasks,
                    task_metrics.enqueued_total,
                    task_metrics.dequeued_total,
                    task_metrics.ready_total,
                    task_metrics.finished_total,
                    task_metrics.failed_total,
                    task_metrics.cancelled_total,
                    task_metrics.stale_dropped_total,
                    task_metrics.queue_dropped_total
                ));
                log.last_snapshot = now;
                log.last_flush = Instant::now();
            }
        }

        let frame_sleep = if processed_input_event {
            Duration::from_millis(1)
        } else {
            Duration::from_millis(16)
        };
        tokio::time::sleep(frame_sleep).await;
    }
}

async fn auto_refresh_loop(
    action_tx: mpsc::UnboundedSender<Action>,
    mut shutdown_rx: broadcast::Receiver<()>,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(5));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = shutdown_rx.recv() => break,
            _ = interval.tick() => {
                let _ = action_tx.send(Action::System(SystemAction::AutoRefresh));
            }
        }
    }
}

async fn dispatch_loop(
    mut app: app::AppState,
    perf: Arc<PerfCounters>,
    mut action_rx: mpsc::UnboundedReceiver<Action>,
    command_tx: mpsc::UnboundedSender<app::Command>,
    state_version_tx: watch::Sender<u64>,
    snapshot_tx: watch::Sender<Arc<AppStateSnapshotOwned>>,
    mut shutdown_rx: broadcast::Receiver<()>,
) {
    let mut dispatcher = flux::dispatcher::Dispatcher::with_default_stores();
    loop {
        tokio::select! {
            _ = shutdown_rx.recv() => break,
            maybe_action = action_rx.recv() => {
                let Some(action) = maybe_action else { break; };
                perf.action_dequeued.fetch_add(1, Ordering::Relaxed);
                update_backlog(
                    &perf.action_enqueued,
                    &perf.action_dequeued,
                    &perf.action_backlog_max,
                );
                let started = Instant::now();
                let reduce_started = Instant::now();
                let envelope = dispatcher.next_envelope(action);
                let result = dispatcher.dispatch(&mut app, envelope);
                let reduce_us = reduce_started.elapsed().as_micros() as u64;

                if app.ui.dirty.is_dirty() {
                    if app.ui.dirty.main_content() {
                        app.refresh_render_cache();
                    }
                    let snapshot = Arc::new(AppStateSnapshotOwned::from_app(&app));
                    app.ui.dirty.clear();
                    let _ = snapshot_tx.send(snapshot);
                }

                let elapsed_us = started.elapsed().as_micros() as u64;
                perf.dispatch_actions.fetch_add(1, Ordering::Relaxed);
                perf.dispatch_us.fetch_add(elapsed_us, Ordering::Relaxed);
                perf.dispatch_reduce_us.fetch_add(reduce_us, Ordering::Relaxed);
                update_max(&perf.dispatch_max_us, elapsed_us);
                perf.dispatch_commands
                    .fetch_add(result.commands.len() as u64, Ordering::Relaxed);

                let _ = state_version_tx.send(result.state_version);
                for command in result.commands {
                    let _ = command_tx.send(command);
                    perf.command_enqueued.fetch_add(1, Ordering::Relaxed);
                    update_backlog(
                        &perf.command_enqueued,
                        &perf.command_dequeued,
                        &perf.command_backlog_max,
                    );
                }
            }
        }
    }
}

async fn effect_loop(
    runtime: app::AppRuntime,
    perf: Arc<PerfCounters>,
    mut command_rx: mpsc::UnboundedReceiver<app::Command>,
    action_tx: mpsc::UnboundedSender<Action>,
    mut shutdown_rx: broadcast::Receiver<()>,
) {
    let mut executor = EffectExecutor::new(runtime);
    loop {
        let actions = tokio::select! {
            _ = shutdown_rx.recv() => break,
            completion = executor.next_completed(), if executor.has_jobs() => {
                perf.effect_us.fetch_add(completion.elapsed_us, Ordering::Relaxed);
                completion.actions
            }
            maybe_command = command_rx.recv() => {
                let Some(command) = maybe_command else { break; };
                perf.command_dequeued.fetch_add(1, Ordering::Relaxed);
                update_backlog(&perf.command_enqueued, &perf.command_dequeued, &perf.command_backlog_max);
                perf.effect_commands.fetch_add(1, Ordering::Relaxed);
                match command {
                    app::Command::None => Vec::new(),
                    app::Command::Sync(action) => vec![Action::Domain(action)],
                    app::Command::Effect(request) => executor.enqueue(request),
                }
            }
        };
        perf.effect_actions_emitted
            .fetch_add(actions.len() as u64, Ordering::Relaxed);
        for action in actions {
            let _ = action_tx.send(action);
            perf.action_enqueued.fetch_add(1, Ordering::Relaxed);
            update_backlog(
                &perf.action_enqueued,
                &perf.action_dequeued,
                &perf.action_backlog_max,
            );
        }
    }
}
