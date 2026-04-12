mod app;
mod backend;
mod components;
mod shared;

use anyhow::Result;
use app::App;
use backend::{BackendCommand, CommandEnvelope, EventEnvelope};
use tokio::sync::mpsc;

const CHANNEL_CAPACITY: usize = 100;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let (cmd_tx, cmd_rx) = mpsc::channel::<CommandEnvelope>(CHANNEL_CAPACITY);
    let (event_tx, event_rx) = mpsc::channel::<EventEnvelope>(CHANNEL_CAPACITY);

    let backend_handle = tokio::spawn(backend::run_backend(cmd_rx, event_tx));
    let result = App::new(cmd_tx.clone(), event_rx).run().await;

    let _ = cmd_tx
        .send(CommandEnvelope::new(0, BackendCommand::Quit))
        .await;
    backend_handle.await?;

    result
}
