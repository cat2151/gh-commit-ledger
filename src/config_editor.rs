use std::io::ErrorKind;
use std::process::{Command, ExitStatus};

use anyhow::{Context, Result, anyhow, bail};

use crate::terminal::{self, AppTerminal};

pub(crate) fn edit_config_file(
    terminal: &mut AppTerminal,
    config_path: &str,
    editors: &[String],
) -> Result<()> {
    terminal::suspend(terminal)?;
    let edit_result = run_config_editor(config_path, editors);
    let resume_result = terminal::resume(terminal);

    match (edit_result, resume_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(edit_error), Ok(())) => Err(edit_error),
        (Ok(()), Err(resume_error)) => Err(resume_error),
        (Err(edit_error), Err(resume_error)) => Err(anyhow!(
            "{edit_error}; TUI への復帰にも失敗しました: {resume_error}"
        )),
    }
}

fn run_config_editor(config_path: &str, editors: &[String]) -> Result<()> {
    let mut attempted = Vec::new();

    for editor in editors.iter().map(|editor| editor.trim()) {
        if editor.is_empty() {
            continue;
        }
        attempted.push(editor.to_string());

        let mut command = Command::new(editor);
        if editor.eq_ignore_ascii_case("code") {
            command.arg("--wait");
        }
        command.arg(config_path);

        match command.status() {
            Ok(status) if status.success() => return Ok(()),
            Ok(status) => bail!(
                "editor `{editor}` が終了コード {} で終了しました",
                format_exit_status(status)
            ),
            Err(error) if error.kind() == ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("editor `{editor}` を起動できませんでした"));
            }
        }
    }

    if attempted.is_empty() {
        bail!("config の editors に有効な editor がありません");
    }

    bail!(
        "利用できる editor が見つかりませんでした: {}",
        attempted.join(", ")
    )
}

fn format_exit_status(status: ExitStatus) -> String {
    status
        .code()
        .map(|code| code.to_string())
        .unwrap_or_else(|| "不明".to_string())
}
