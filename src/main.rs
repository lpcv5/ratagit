mod app;
mod backend;
mod components;
mod shared;

use anyhow::Result;
use app::App;
use backend::{BackendCommand, CommandEnvelope, EventEnvelope};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<CommandEnvelope>();
    let (event_tx, event_rx) = mpsc::unbounded_channel::<EventEnvelope>();

    let backend_handle = tokio::spawn(backend::run_backend(cmd_rx, event_tx));
    let result = App::new(cmd_tx.clone(), event_rx).run().await;

    let _ = cmd_tx.send(CommandEnvelope::new(0, BackendCommand::Quit));
    backend_handle.await?;

    result
}
