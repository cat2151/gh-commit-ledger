use anyhow::{Context, Result};
use arboard::Clipboard;

use crate::report::ReportOutcome;

pub(crate) fn copy_report(report: &ReportOutcome, message_template: &str) -> Result<()> {
    let mut clipboard = Clipboard::new().context("clipboard を初期化できませんでした")?;
    clipboard
        .set_text(report.report.clipboard_message(message_template))
        .context("clipboard への書き込みに失敗しました")?;
    Ok(())
}
