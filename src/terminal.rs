//! Terminal resources and restoration, independent of application scheduling.
use color_eyre::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
};
use ratatui::DefaultTerminal;
use std::io;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tracing::{error, info};

pub(crate) trait TerminalOps {
    type Terminal;
    fn init(&mut self) -> io::Result<Self::Terminal>;
    fn install_panic_hook(&mut self);
    fn enable_mouse(&mut self) -> io::Result<()>;
    fn disable_mouse(&mut self) -> io::Result<()>;
    fn restore(&mut self) -> io::Result<()>;
    fn restore_failed_init(&mut self) {
        let _ = self.restore();
    }
    fn restored_on_panic(&self) -> bool {
        false
    }
}

#[derive(Default)]
pub(crate) struct CrosstermOps {
    restored_on_panic: Arc<AtomicBool>,
}

impl TerminalOps for CrosstermOps {
    type Terminal = DefaultTerminal;
    fn init(&mut self) -> io::Result<DefaultTerminal> {
        ratatui::try_init()
    }
    fn restore_failed_init(&mut self) {
        ratatui::restore();
    }
    fn install_panic_hook(&mut self) {
        // Ratatui restores raw mode and the alternate screen before color-eyre.
        // Application-specific mouse capture must end before diagnostics too.
        let hook = std::panic::take_hook();
        let restored = self.restored_on_panic.clone();
        std::panic::set_hook(Box::new(move |info| {
            let _ = execute!(io::stdout(), DisableMouseCapture);
            restored.store(true, Ordering::Relaxed);
            hook(info);
        }));
    }
    fn enable_mouse(&mut self) -> io::Result<()> {
        execute!(io::stdout(), EnableMouseCapture)
    }
    fn disable_mouse(&mut self) -> io::Result<()> {
        execute!(io::stdout(), DisableMouseCapture)
    }
    fn restore(&mut self) -> io::Result<()> {
        ratatui::try_restore()
    }
    fn restored_on_panic(&self) -> bool {
        self.restored_on_panic.load(Ordering::Relaxed)
    }
}

pub(crate) struct TerminalSession<O: TerminalOps = CrosstermOps> {
    ops: O,
    terminal: Option<O::Terminal>,
}

impl TerminalSession {
    pub fn new() -> Result<Self> {
        Self::with_ops(CrosstermOps::default())
    }
}

impl<O: TerminalOps> TerminalSession<O> {
    fn with_ops(mut ops: O) -> Result<Self> {
        let terminal = match ops.init() {
            Ok(terminal) => terminal,
            Err(error) => {
                ops.restore_failed_init();
                return Err(error.into());
            }
        };
        ops.install_panic_hook();
        Ok(Self {
            ops,
            terminal: Some(terminal),
        })
    }

    pub fn enable_mouse(&mut self) -> io::Result<()> {
        self.ops.enable_mouse()
    }
    pub fn terminal_mut(&mut self) -> &mut O::Terminal {
        self.terminal.as_mut().expect("terminal session is active")
    }

    fn cleanup(&mut self) -> (io::Result<()>, io::Result<()>) {
        let Some(terminal) = self.terminal.take() else {
            return (Ok(()), Ok(()));
        };
        let results = if std::thread::panicking() && self.ops.restored_on_panic() {
            // The chained panic hook has already performed restoration before
            // diagnostics. Do not emit another cleanup sequence during unwind.
            (Ok(()), Ok(()))
        } else {
            let mouse = self.ops.disable_mouse();
            let restore = self.ops.restore();
            (mouse, restore)
        };
        drop(terminal);
        results
    }

    /// Attempt both cleanup steps, then preserve application/mouse/restore error
    /// priority. The terminal is dropped before reporting the final outcome.
    pub fn finish(mut self, result: Result<()>) -> Result<()> {
        let (mouse, restore) = self.cleanup();
        if let Err(error) = &result {
            error!("app_error err={:?}", error);
        }
        info!(
            "app_exit ok={}",
            result.is_ok() && mouse.is_ok() && restore.is_ok()
        );
        result?;
        mouse?;
        restore?;
        Ok(())
    }
}

impl<O: TerminalOps> Drop for TerminalSession<O> {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

#[cfg(test)]
mod tests;
