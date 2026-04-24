use std::io::{self, Stdout};

use anyhow::{Context, Result};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

pub(crate) type AppTerminal = Terminal<CrosstermBackend<Stdout>>;

pub(crate) fn setup() -> Result<AppTerminal> {
    enable_raw_mode().context("raw mode を有効化できませんでした")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).context("代替スクリーンへ切り替えられませんでした")?;
    Terminal::new(CrosstermBackend::new(stdout)).context("terminal を初期化できませんでした")
}

pub(crate) fn suspend(terminal: &mut AppTerminal) -> Result<()> {
    disable_raw_mode().context("raw mode の解除に失敗しました")?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)
        .context("代替スクリーンの終了に失敗しました")?;
    terminal
        .show_cursor()
        .context("カーソルを再表示できませんでした")
}

pub(crate) fn resume(terminal: &mut AppTerminal) -> Result<()> {
    enable_raw_mode().context("raw mode を有効化できませんでした")?;
    execute!(terminal.backend_mut(), EnterAlternateScreen)
        .context("代替スクリーンへ切り替えられませんでした")?;
    terminal
        .clear()
        .context("terminal を再描画できませんでした")
}

pub(crate) fn restore(terminal: &mut AppTerminal) -> Result<()> {
    disable_raw_mode().context("raw mode の解除に失敗しました")?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)
        .context("代替スクリーンの終了に失敗しました")?;
    terminal
        .show_cursor()
        .context("カーソルを再表示できませんでした")
}
