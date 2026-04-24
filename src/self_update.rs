use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckResult {
    pub embedded_hash: String,
    pub remote_hash: String,
}

impl CheckResult {
    pub fn is_up_to_date(&self) -> bool {
        self.embedded_hash == self.remote_hash
    }
}

impl fmt::Display for CheckResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let result = if self.is_up_to_date() {
            "up-to-date"
        } else {
            "update available"
        };

        write!(
            f,
            "embedded: {}\nremote: {}\nresult: {}",
            self.embedded_hash, self.remote_hash, result
        )
    }
}

pub fn compare_hashes(embedded_hash: &str, remote_hash: &str) -> CheckResult {
    CheckResult {
        embedded_hash: embedded_hash.to_string(),
        remote_hash: remote_hash.to_string(),
    }
}

pub fn check_remote_commit(
    owner: &str,
    repo: &str,
    branch: &str,
    embedded_hash: &str,
) -> Result<CheckResult, Box<dyn std::error::Error>> {
    let remote_hash = fetch_remote_branch_head(owner, repo, branch)?;
    Ok(compare_hashes(embedded_hash, &remote_hash))
}

/// Start a self-update in a detached helper process.
///
/// The helper waits for this process to exit on Windows, then runs
/// `cargo install --force --git https://github.com/<owner>/<repo>`.
pub fn self_update(
    owner: &str,
    repo: &str,
    crates: &[&str],
) -> Result<(), Box<dyn std::error::Error>> {
    let py_content = generate_py_script(owner, repo, crates, std::process::id());
    let py_path = unique_tmp_path();

    fs::write(&py_path, &py_content)?;
    spawn_python(&py_path)?;

    Ok(())
}

fn escape_py_single_quoted(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\'' => out.push_str("\\'"),
            _ => out.push(ch),
        }
    }
    out
}

fn generate_py_script(owner: &str, repo: &str, crates: &[&str], parent_pid: u32) -> String {
    let repo_url = format!("https://github.com/{owner}/{repo}");
    let repo_url_escaped = escape_py_single_quoted(&repo_url);

    let install_parts = if crates.is_empty() {
        format!(
            "['cargo', 'install', '--force', '--git', '{}']",
            repo_url_escaped
        )
    } else {
        let crate_args = crates
            .iter()
            .map(|crate_name| format!("'{}'", escape_py_single_quoted(crate_name)))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "['cargo', 'install', '--force', '--git', '{}', {}]",
            repo_url_escaped, crate_args
        )
    };

    format!(
        r#"import subprocess
import os
import shlex
import sys
import traceback

PARENT_PID = {parent_pid}
INSTALL_PARTS = {install_parts}

def log(message):
    print(message, flush=True)

def format_command(parts):
    if sys.platform == 'win32':
        return subprocess.list2cmdline(parts)
    return shlex.join(parts)

def wait_for_parent_exit():
    if sys.platform != 'win32':
        return

    import ctypes

    SYNCHRONIZE = 0x00100000
    INFINITE = 0xFFFFFFFF
    kernel32 = ctypes.windll.kernel32
    handle = kernel32.OpenProcess(SYNCHRONIZE, False, PARENT_PID)
    if not handle:
        return

    try:
        kernel32.WaitForSingleObject(handle, INFINITE)
    finally:
        kernel32.CloseHandle(handle)

def wait_for_user_acknowledgement():
    if sys.platform != 'win32':
        return

    log("Enterキーを押すと閉じます")
    try:
        input()
    except EOFError:
        pass

try:
    log("現在のプロセスの終了を待っています")
    wait_for_parent_exit()
    log("cargo installを起動しています")
    log(f"$ {{format_command(INSTALL_PARTS)}}")
    subprocess.run(INSTALL_PARTS, check=True)
    log("cargo install が完了しました")
except subprocess.CalledProcessError as err:
    log(f"更新に失敗しました。終了コード: {{err.returncode}}")
    wait_for_user_acknowledgement()
    sys.exit(err.returncode)
except Exception as err:
    log(f"更新処理に失敗しました: {{err}}")
    traceback.print_exc()
    wait_for_user_acknowledgement()
    sys.exit(1)
finally:
    try:
        os.remove(__file__)
    except OSError:
        pass
"#,
        parent_pid = parent_pid,
        install_parts = install_parts,
    )
}

fn fetch_remote_branch_head(
    owner: &str,
    repo: &str,
    branch: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let repo_url = format!("https://github.com/{owner}/{repo}");
    let ref_name = format!("refs/heads/{branch}");
    let output = git_command_without_prompt()
        .args(["ls-remote", repo_url.as_str(), ref_name.as_str()])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let message = if stderr.is_empty() {
            format!("git ls-remote failed with status {}", output.status)
        } else {
            format!("git ls-remote failed: {stderr}")
        };
        return Err(message.into());
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|err| format!("git ls-remote returned invalid UTF-8 output: {err}"))?;
    parse_ls_remote_hash(&stdout, &ref_name)
        .ok_or_else(|| format!("could not find remote hash for {ref_name}").into())
}

fn git_command_without_prompt() -> Command {
    let mut command = Command::new("git");
    command
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_ASKPASS", "");
    command
}

fn parse_ls_remote_hash(output: &str, ref_name: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        let hash = parts.next()?;
        let name = parts.next()?;
        if name != ref_name {
            return None;
        }

        if parts.next().is_some() {
            return None;
        }

        Some(hash.to_string())
    })
}

fn unique_tmp_path() -> PathBuf {
    let pid = std::process::id();
    let timestamp_nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let filename = format!("gh_commit_ledger_self_update_{pid}_{timestamp_nanos}.py");
    std::env::temp_dir().join(filename)
}

fn spawn_python(py_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;

        const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
        Command::new("python")
            .arg(py_path)
            .creation_flags(CREATE_NEW_CONSOLE)
            .spawn()?;
    }

    #[cfg(not(windows))]
    {
        Command::new("python3").arg(py_path).spawn()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compare_hashes_reports_up_to_date() {
        let result = compare_hashes("abc", "abc");
        assert!(result.is_up_to_date());
        assert_eq!(
            result.to_string(),
            "embedded: abc\nremote: abc\nresult: up-to-date"
        );
    }

    #[test]
    fn compare_hashes_reports_update_available() {
        let result = compare_hashes("abc", "def");
        assert!(!result.is_up_to_date());
        assert_eq!(
            result.to_string(),
            "embedded: abc\nremote: def\nresult: update available"
        );
    }

    #[test]
    fn py_script_contains_cargo_install_from_repo() {
        let script = generate_py_script("cat2151", "gh-commit-ledger", &[], 1234);
        assert!(script.contains("https://github.com/cat2151/gh-commit-ledger"));
        assert!(script.contains("['cargo', 'install', '--force', '--git'"));
        assert!(script.contains("PARENT_PID = 1234"));
    }

    #[test]
    fn py_script_installs_specified_crates() {
        let script = generate_py_script("owner", "repo", &["bin-a", "bin-b"], 1234);
        assert!(script.contains(
            "INSTALL_PARTS = ['cargo', 'install', '--force', '--git', 'https://github.com/owner/repo', 'bin-a', 'bin-b']"
        ));
    }

    #[test]
    fn py_string_escape_handles_single_quotes_and_backslashes() {
        assert_eq!(escape_py_single_quoted("a'b"), "a\\'b");
        assert_eq!(escape_py_single_quoted("a\\b"), "a\\\\b");
    }

    #[test]
    fn parse_ls_remote_hash_finds_matching_branch() {
        let output = "abc123\trefs/heads/main\ndef456\trefs/heads/feature\n";
        assert_eq!(
            parse_ls_remote_hash(output, "refs/heads/main"),
            Some("abc123".to_string())
        );
    }

    #[test]
    fn parse_ls_remote_hash_returns_none_for_missing_branch() {
        let output = "abc123\trefs/heads/main\n";
        assert_eq!(parse_ls_remote_hash(output, "refs/heads/release"), None);
    }

    #[test]
    fn parse_ls_remote_hash_rejects_extra_fields() {
        let output = "abc123\trefs/heads/main\textra\n";
        assert_eq!(parse_ls_remote_hash(output, "refs/heads/main"), None);
    }

    #[test]
    fn unique_tmp_path_has_expected_name() {
        let path = unique_tmp_path();
        let name = path.file_name().unwrap().to_str().unwrap();
        assert!(name.starts_with("gh_commit_ledger_self_update_"));
        assert!(name.ends_with(".py"));
    }
}
