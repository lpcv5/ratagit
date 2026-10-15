//! Action reduction, FIFO command routing and periodic refresh scheduling.
use super::telemetry::{update_backlog, update_max, PerfCounters};
use crate::flux::action::{Action, SystemAction};
use crate::flux::runtime::EffectExecutor;
use crate::flux::snapshot::AppStateSnapshotOwned;
use crate::{app, flux};
use std::sync::{atomic::Ordering, Arc};
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc, watch};

pub(super) async fn auto_refresh_loop(
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

pub(super) async fn dispatch_loop(
    (mut app, initial_output): (app::AppState, flux::command_output::CommandOutput),
    perf: Arc<PerfCounters>,
    mut action_rx: mpsc::UnboundedReceiver<Action>,
    command_tx: mpsc::UnboundedSender<app::Command>,
    state_version_tx: watch::Sender<u64>,
    snapshot_tx: watch::Sender<Arc<AppStateSnapshotOwned>>,
    mut shutdown_rx: broadcast::Receiver<()>,
) {
    let mut dispatcher = flux::dispatcher::Dispatcher::with_initial_output(initial_output);
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

pub(super) async fn command_loop(
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
                flux::runtime::route_command(command, &mut executor)
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
