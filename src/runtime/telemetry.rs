//! Logging and performance counters used by runtime loops.
use std::fs::{File, OpenOptions};
use std::io;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

pub(super) const ENABLE_DEBUG_LOG: bool = cfg!(debug_assertions);
const APP_LOG_FILE: &str = "ratagit.log";

pub(crate) fn init_logging(debug_mode: bool) {
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
pub(super) struct PerfCounters {
    pub(super) ui_events: AtomicU64,
    pub(super) ui_messages: AtomicU64,
    pub(super) ui_ticks_sent: AtomicU64,
    pub(super) ui_draws: AtomicU64,
    pub(super) ui_draw_us: AtomicU64,
    pub(super) dispatch_actions: AtomicU64,
    pub(super) dispatch_commands: AtomicU64,
    pub(super) dispatch_us: AtomicU64,
    pub(super) dispatch_lock_wait_us: AtomicU64,
    pub(super) dispatch_reduce_us: AtomicU64,
    pub(super) effect_commands: AtomicU64,
    pub(super) effect_actions_emitted: AtomicU64,
    pub(super) effect_us: AtomicU64,
    pub(super) action_enqueued: AtomicU64,
    pub(super) action_dequeued: AtomicU64,
    pub(super) action_backlog_max: AtomicU64,
    pub(super) command_enqueued: AtomicU64,
    pub(super) command_dequeued: AtomicU64,
    pub(super) command_backlog_max: AtomicU64,
    pub(super) ui_lock_wait_us: AtomicU64,
    pub(super) ui_key_handle_us: AtomicU64,
    pub(super) ui_key_handle_max_us: AtomicU64,
    pub(super) ui_draw_max_us: AtomicU64,
    pub(super) dispatch_max_us: AtomicU64,
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

pub(super) fn update_max(metric: &AtomicU64, value: u64) {
    let mut prev = metric.load(Ordering::Relaxed);
    while value > prev {
        match metric.compare_exchange_weak(prev, value, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(actual) => prev = actual,
        }
    }
}

pub(super) fn update_backlog(enqueued: &AtomicU64, dequeued: &AtomicU64, backlog_max: &AtomicU64) {
    let enq = enqueued.load(Ordering::Relaxed);
    let deq = dequeued.load(Ordering::Relaxed);
    let backlog = enq.saturating_sub(deq);
    update_max(backlog_max, backlog);
}

pub(super) struct PerfLog {
    file: File,
    last_flush: Instant,
    last_snapshot: PerfSnapshot,
}

impl PerfLog {
    pub(super) fn new() -> io::Result<Self> {
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

    pub(super) fn write_line(&mut self, line: &str) {
        use std::io::Write;
        let _ = writeln!(self.file, "{}", line);
        let _ = self.file.flush();
    }
}

impl PerfLog {
    pub(super) fn record(&mut self, perf: &PerfCounters) {
        if self.last_flush.elapsed() >= Duration::from_secs(1) {
            let now = perf.snapshot();
            let prev = self.last_snapshot;
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
            self.write_line(&format!(
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
            self.last_snapshot = now;
            self.last_flush = Instant::now();
        }
    }
}
