mod app;
mod backend;
mod components;
mod git;

use anyhow::Result;
use app::App;
use backend::{run_backend, BackendCommand, FrontendEvent};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<BackendCommand>();
    let (event_tx, event_rx) = mpsc::unbounded_channel::<FrontendEvent>();

    let backend_handle = tokio::spawn(run_backend(cmd_rx, event_tx));
    let result = App::new(cmd_tx.clone(), event_rx).run().await;

    let _ = cmd_tx.send(BackendCommand::Quit);
    backend_handle.await?;

    result
}
