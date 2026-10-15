//! Action reduction, FIFO command routing and periodic refresh scheduling.
use super::telemetry::{update_backlog, update_max, PerfCounters};
use super::tracking::{ActionSender, PublishedSnapshot, Tracked};
use crate::background::BackgroundLoad;
use crate::flux::action::{Action, DomainAction, RuntimeEvent, SystemAction};
use crate::flux::effects::EffectRequest;
use crate::flux::runtime::EffectExecutor;
use crate::flux::snapshot::AppStateSnapshotOwned;
use crate::{app, flux};
use std::sync::{atomic::Ordering, Arc};
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc, watch};

pub(super) async fn auto_refresh_loop(
    action_tx: ActionSender,
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

pub(super) async fn dispatch_loop(
    (mut app, initial_output): (app::AppState, flux::command_output::CommandOutput),
    perf: Arc<PerfCounters>,
    mut action_rx: mpsc::UnboundedReceiver<Tracked<Action>>,
    command_tx: mpsc::UnboundedSender<Tracked<app::Command>>,
    state_version_tx: watch::Sender<u64>,
    snapshot_tx: watch::Sender<Arc<PublishedSnapshot>>,
    mut shutdown_rx: broadcast::Receiver<()>,
) {
    let mut dispatcher = flux::dispatcher::Dispatcher::with_initial_output(initial_output);
    let mut diff_origin = None;
    let mut refresh_origin = None;
    loop {
        tokio::select! {
            _ = shutdown_rx.recv() => break,
            maybe_action = action_rx.recv() => {
                let Some(message) = maybe_action else { break; };
                message.dequeued("action_dequeued");
                let Tracked { value: action, trace, .. } = message;
                let pending_diff = app.pending_diff_reload_at();
                let pending_refresh = app.pending_refresh_kind();
                let refresh_requested = matches!(
                    &action,
                    Action::Domain(DomainAction::Refresh)
                        | Action::System(SystemAction::AutoRefresh)
                );
                let process_loads = matches!(
                    &action,
                    Action::System(SystemAction::Tick)
                        | Action::Domain(DomainAction::Runtime(
                            RuntimeEvent::ProcessBackgroundLoads
                                | RuntimeEvent::FlushPendingRefresh { .. }
                        ))
                );
                let current_completion = match &action {
                    Action::System(SystemAction::BackgroundCompleted(completed)) => {
                        let current = trace.is_none() || app.background_completion_is_current(completed);
                        if let Some(trace) = &trace {
                            let stage = if !current {
                                "completion_rejected"
                            } else if completed.result.is_err() {
                                "completion_failed"
                            } else {
                                "completion_current"
                            };
                            trace.event(stage, Duration::ZERO);
                        }
                        current
                    }
                    _ => true,
                };
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
                if perf.logging_enabled() {
                    perf.diff_cache.record(app.diff.cache_stats());
                }

                if refresh_requested
                    || (pending_refresh != app.pending_refresh_kind()
                        && app.pending_refresh_kind().is_some())
                {
                    refresh_origin = trace.clone().or(refresh_origin);
                }
                if app.pending_diff_reload_at().is_some()
                    && app.pending_diff_reload_at() != pending_diff
                {
                    diff_origin = trace.clone().or_else(|| refresh_origin.clone());
                }
                if let Some(trace) = &trace {
                    trace.event("dispatch_finished", reduce_started.elapsed());
                }

                if app.ui.dirty.is_dirty() {
                    let snapshot_started = Instant::now();
                    if app.ui.dirty.main_content() {
                        app.refresh_render_cache();
                    }
                    let state = AppStateSnapshotOwned::from_app(&app);
                    let publication_trace = if pending_diff.is_some()
                        && !app.has_pending_diff_reload()
                        && !app.diff.is_loading()
                    {
                        diff_origin.clone().or_else(|| trace.clone())
                            .map(|origin| origin.child("diff_ready", None))
                    } else {
                        trace.clone()
                    }.filter(|_| current_completion);
                    if let Some(trace) = &publication_trace {
                        trace.event("snapshot_built", snapshot_started.elapsed());
                    }
                    let snapshot = Arc::new(PublishedSnapshot::new(
                        state, result.state_version, publication_trace,
                    ));
                    app.ui.dirty.clear();
                    snapshot_tx.borrow().finish("snapshot_superseded", Duration::ZERO);
                    let _ = snapshot_tx.send(snapshot);
                } else if let Some(trace) = &trace {
                    trace.event("dispatch_no_snapshot", Duration::ZERO);
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
                    let origin = match &command {
                        app::Command::Effect(EffectRequest::LoadBackground { request })
                            if matches!(&request.load, BackgroundLoad::Diff { .. }) => {
                                diff_origin.clone().or_else(|| trace.clone())
                            }
                        app::Command::Sync(DomainAction::Runtime(
                            RuntimeEvent::FlushPendingDiffReload,
                        )) => diff_origin.clone().or_else(|| trace.clone()),
                        app::Command::Sync(DomainAction::Runtime(
                            RuntimeEvent::ProcessBackgroundLoads,
                        )) => refresh_origin.clone().or_else(|| trace.clone()),
                        app::Command::Effect(EffectRequest::LoadBackground { .. })
                            if process_loads => refresh_origin.clone().or_else(|| trace.clone()),
                        _ => trace.clone(),
                    };
                    let command_trace = match &command {
                        app::Command::Effect(request) => origin.map(|origin| {
                            let (operation, generation) = request.diagnostic_identity();
                            origin.child(operation, generation)
                        }),
                        _ => origin,
                    };
                    if let Some(trace) = &command_trace {
                        trace.event("command_queued", Duration::ZERO);
                    }
                    let _ = command_tx.send(Tracked::new(command, command_trace));
                    perf.command_enqueued.fetch_add(1, Ordering::Relaxed);
                    update_backlog(
                        &perf.command_enqueued,
                        &perf.command_dequeued,
                        &perf.command_backlog_max,
                    );
                }
                if !app.has_pending_diff_reload() {
                    diff_origin = None;
                }
                if !app.has_pending_refresh_work() {
                    refresh_origin = None;
                }
            }
        }
    }
}

pub(super) async fn command_loop(
    runtime: app::AppRuntime,
    perf: Arc<PerfCounters>,
    mut command_rx: mpsc::UnboundedReceiver<Tracked<app::Command>>,
    action_tx: ActionSender,
    mut shutdown_rx: broadcast::Receiver<()>,
    editor_tx: mpsc::UnboundedSender<crate::desktop::EditorCommand>,
) {
    let mut executor = EffectExecutor::new(runtime);
    loop {
        let actions = tokio::select! {
            _ = shutdown_rx.recv() => break,
            completion = executor.next_completed(), if executor.has_jobs() => {
                perf.effect_us.fetch_add(completion.elapsed_us, Ordering::Relaxed);
                (completion.actions, completion.trace)
            }
            maybe_command = command_rx.recv() => {
                let Some(message) = maybe_command else { break; };
                message.dequeued("command_dequeued");
                let Tracked { value: command, trace, .. } = message;
                perf.command_dequeued.fetch_add(1, Ordering::Relaxed);
                update_backlog(&perf.command_enqueued, &perf.command_dequeued, &perf.command_backlog_max);
                perf.effect_commands.fetch_add(1, Ordering::Relaxed);
                let actions = match command {
                    app::Command::RunEditor(command) => {
                        match editor_tx.send(command) {
                            Ok(()) => Vec::new(),
                            Err(_) => vec![Action::Domain(DomainAction::EditorFinished {
                                result: Err(crate::git::GitError::Worker("terminal UI unavailable".into())),
                            })],
                        }
                    }
                    app::Command::Effect(request) => {
                        executor.enqueue_traced(request, trace.clone());
                        Vec::new()
                    }
                    other => flux::runtime::route_command(other, &mut executor),
                };
                (actions, trace)
            }
        };
        let (actions, trace) = actions;
        perf.effect_actions_emitted
            .fetch_add(actions.len() as u64, Ordering::Relaxed);
        for action in actions {
            let _ = action_tx.send_traced(action, trace.clone());
            perf.action_enqueued.fetch_add(1, Ordering::Relaxed);
            update_backlog(
                &perf.action_enqueued,
                &perf.action_dequeued,
                &perf.action_backlog_max,
            );
        }
    }
}
