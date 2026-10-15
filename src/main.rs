mod app;
mod background;
mod config;
mod file_tree;
mod flux;
mod git;
mod runtime;
mod shared;
mod terminal;
mod ui;

use app::App;
use color_eyre::Result;
use terminal::TerminalSession;
use tracing::info;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let debug_mode = std::env::args().any(|arg| arg == "--debug");
    runtime::init_logging(debug_mode);
    info!("app_start debug={}", debug_mode);
    if debug_mode {
        git::enable_git_job_log("ratagit-git-jobs.log");
    }

    // Repository/configuration errors must precede terminal initialization.
    let app = App::new()?;
    let mut session = TerminalSession::new()?;
    let result = async {
        session.enable_mouse()?;
        runtime::run(session.terminal_mut(), app).await
    }
    .await;
    session.finish(result)
}
