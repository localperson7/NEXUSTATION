use std::io::{self, Stdout, Write};
use std::panic;
use std::sync::Once;

use crossterm::{
    cursor::{Hide, Show},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

static PANIC_HOOK_ONCE: Once = Once::new();

/// Abstraction over terminal mode transitions so cleanup guarantees can be
/// verified deterministically in headless tests as well as on real TTYs.
pub trait LifecycleOps {
    fn enter(&mut self) -> io::Result<()>;
    fn leave(&mut self) -> io::Result<()>;
}

/// Production Crossterm terminal mode controller.
pub struct CrosstermOps;

impl LifecycleOps for CrosstermOps {
    fn enter(&mut self) -> io::Result<()> {
        enable_raw_mode()?;
        let mut out = io::stdout();
        if let Err(err) = execute!(out, EnterAlternateScreen, Hide) {
            let _ = disable_raw_mode();
            return Err(err);
        }
        Ok(())
    }

    fn leave(&mut self) -> io::Result<()> {
        let mut out = io::stdout();
        let screen_res = execute!(out, LeaveAlternateScreen, Show);
        let _ = out.flush();
        let raw_res = disable_raw_mode();
        screen_res.and(raw_res)
    }
}

/// Writes Crossterm ANSI screen/cursor sequences to an arbitrary `Write` sink (used for verification).
pub fn write_enter_sequences<W: Write>(writer: &mut W) -> io::Result<()> {
    execute!(writer, EnterAlternateScreen, Hide)
}

/// Writes Crossterm ANSI cleanup sequences to an arbitrary `Write` sink.
pub fn write_leave_sequences<W: Write>(writer: &mut W) -> io::Result<()> {
    execute!(writer, LeaveAlternateScreen, Show)?;
    writer.flush()
}

/// RAII guard that guarantees terminal state is restored on normal exit,
/// early `?` error returns, and unwinding panics.
pub struct TerminalGuard<O: LifecycleOps> {
    ops: O,
    active: bool,
}

impl<O: LifecycleOps> TerminalGuard<O> {
    pub fn acquire(mut ops: O) -> io::Result<Self> {
        ops.enter()?;
        Ok(Self { ops, active: true })
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Explicitly restores terminal state; safe to call multiple times.
    pub fn restore(&mut self) -> io::Result<()> {
        if self.active {
            self.active = false;
            self.ops.leave()?;
        }
        Ok(())
    }

    pub fn ops(&self) -> &O {
        &self.ops
    }
}

impl<O: LifecycleOps> Drop for TerminalGuard<O> {
    fn drop(&mut self) {
        if self.active {
            self.active = false;
            let _ = self.ops.leave();
        }
    }
}

/// Installs a global panic hook once so any panic restores raw mode and alternate screen
/// before delegating to the standard Rust panic formatter.
pub fn install_panic_hook() {
    PANIC_HOOK_ONCE.call_once(|| {
        let prev = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            let mut ops = CrosstermOps;
            let _ = ops.leave();
            prev(info);
        }));
    });
}

/// High-level terminal session bundling Ratatui `Terminal` and our RAII `TerminalGuard`.
pub struct ConsoleSession {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    guard: TerminalGuard<CrosstermOps>,
}

impl ConsoleSession {
    pub fn enter() -> io::Result<Self> {
        install_panic_hook();
        let guard = TerminalGuard::acquire(CrosstermOps)?;
        let backend = CrosstermBackend::new(io::stdout());
        let mut terminal = Terminal::new(backend)?;
        terminal.clear()?;
        Ok(Self { guard, terminal })
    }

    pub fn terminal_mut(&mut self) -> &mut Terminal<CrosstermBackend<Stdout>> {
        &mut self.terminal
    }

    pub fn shutdown(mut self) -> io::Result<()> {
        let _ = self.terminal.show_cursor();
        self.guard.restore()
    }
}
