mod app;
mod git;
mod ui;

use app::{App, update};
use color_eyre::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::io;

fn main() -> Result<()> {
    // 错误处理设置
    color_eyre::install()?;

    // 设置终端
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 创建应用
    let mut app = App::new()?;

    // 主循环
    let res = run_app(&mut terminal, &mut app);

    // 恢复终端
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {:?}", err);
    }

    Ok(())
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    loop {
        // 渲染
        terminal.draw(|f| {
            app.render(f);
        })?;

        // 事件处理
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                let msg = app.handle_key(key);

                if let Some(msg) = msg {
                    // 更新状态
                    let cmd = update(app, msg);

                    // 处理 Command（Phase 1 先不实现异步，Phase 2 再加）
                    if let Some(_) = cmd {
                        // TODO: Phase 2 处理异步 Command
                    }
                }
            }
        }

        // 检查是否退出
        if !app.running {
            return Ok(());
        }
    }
}
