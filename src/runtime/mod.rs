//! Own runtime tasks and channel wiring; UI never receives mutable AppState.
#[cfg(test)]
mod latency_tests;
mod loops;
mod telemetry;
#[cfg(test)]
mod tests;
mod tracking;
mod ui;

use crate::app;
use crate::app::App;
use crate::flux::action::Action;
use crate::flux::snapshot::AppStateSnapshotOwned;
use color_eyre::Result;
use loops::{auto_refresh_loop, command_loop, dispatch_loop};
use ratatui::DefaultTerminal;
use std::future::Future;
use std::sync::Arc;
pub(crate) use telemetry::init_logging;
use telemetry::PerfCounters;
use tokio::sync::{broadcast, mpsc, watch};
use tracking::{ActionSender, PublishedSnapshot, Tracked};

struct UiChannels {
    action_tx: ActionSender,
    state_version_rx: watch::Receiver<u64>,
    snapshot_rx: watch::Receiver<Arc<PublishedSnapshot>>,
    shutdown_tx: broadcast::Sender<()>,
}

pub(crate) async fn run(
    terminal: &mut DefaultTerminal,
    app: App,
    recorder: Option<Arc<crate::latency::Recorder>>,
) -> Result<()> {
    run_with_ui_recording(app, recorder, |mut channels, perf| async move {
        ui::ui_loop(
            terminal,
            perf,
            channels.action_tx,
            &mut channels.state_version_rx,
            channels.snapshot_rx,
            channels.shutdown_tx,
        )
        .await
    })
    .await
}

#[cfg(test)]
async fn run_with_ui<F, Fut>(app: App, ui: F) -> Result<()>
where
    F: FnOnce(UiChannels, Arc<PerfCounters>) -> Fut,
    Fut: Future<Output = Result<()>>,
{
    run_with_ui_recording(app, None, ui).await
}

async fn run_with_ui_recording<F, Fut>(
    app: App,
    recorder: Option<Arc<crate::latency::Recorder>>,
    ui: F,
) -> Result<()>
where
    F: FnOnce(UiChannels, Arc<PerfCounters>) -> Fut,
    Fut: Future<Output = Result<()>>,
{
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async move {
            let App {
                state,
                runtime,
                initial_output,
            } = app;
            let perf = Arc::new(PerfCounters {
                profiling: recorder.is_some(),
                ..Default::default()
            });
            let (action_tx, action_rx) = mpsc::unbounded_channel::<Tracked<Action>>();
            let action_tx = ActionSender::new(action_tx, recorder);
            let (command_tx, command_rx) = mpsc::unbounded_channel::<Tracked<app::Command>>();
            let (state_version_tx, state_version_rx) = watch::channel(0u64);
            let initial_snapshot = Arc::new(PublishedSnapshot::new(
                AppStateSnapshotOwned::from_app(&state),
                0,
                None,
            ));
            let (snapshot_tx, snapshot_rx) = watch::channel(initial_snapshot);
            let (shutdown_tx, _) = broadcast::channel::<()>(1);

            let dispatch_handle = tokio::task::spawn_local(dispatch_loop(
                (state, initial_output),
                perf.clone(),
                action_rx,
                command_tx,
                state_version_tx,
                snapshot_tx,
                shutdown_tx.subscribe(),
            ));
            let command_handle = tokio::task::spawn_local(command_loop(
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

            let final_snapshot_rx = snapshot_rx.clone();
            let ui_result = ui(
                UiChannels {
                    action_tx,
                    state_version_rx,
                    snapshot_rx,
                    shutdown_tx: shutdown_tx.clone(),
                },
                perf,
            )
            .await;

            let _ = shutdown_tx.send(());
            let _ = dispatch_handle.await;
            let _ = command_handle.await;
            let _ = auto_refresh_handle.await;
            final_snapshot_rx
                .borrow()
                .finish("frame_unobserved", std::time::Duration::ZERO);
            ui_result
        })
        .await
}
