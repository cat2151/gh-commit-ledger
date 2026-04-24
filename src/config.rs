use std::fs;
use std::path::{Path, PathBuf};

#[cfg(test)]
use std::sync::OnceLock;
#[cfg(test)]
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::Deserialize;

pub const DEFAULT_CLIPBOARD_MESSAGE: &str = "\
昨日は {commits_yesterday} 件の commit をしました。
これまでの総 commit 数は {total_commits} 件です。";

const CONFIG_DIR_NAME: &str = "gh-commit-ledger";
const CONFIG_FILE_NAME: &str = "config.toml";
#[cfg(not(test))]
const CONFIG_DIR_ENV: &str = "GH_COMMIT_LEDGER_CONFIG_DIR";
pub const DEFAULT_EDITORS: &[&str] = &["fresh", "zed", "code", "edit", "nano", "vim"];
const DEFAULT_CONFIG_TOML: &str = r#"# gh-commit-ledger config
#
# 利用できるプレースホルダー:
# - {commits_yesterday}: 昨日の commit 数
# - {total_commits}: 総 commit 数

editors = ["fresh", "zed", "code", "edit", "nano", "vim"]

[clipboard]
message = "昨日は {commits_yesterday} 件の commit をしました。\nこれまでの総 commit 数は {total_commits} 件です。"
"#;

#[derive(Debug, Clone)]
pub struct AppConfig {
    path: PathBuf,
    file: ConfigFile,
}

impl AppConfig {
    pub fn load_or_create_default() -> Result<Self> {
        let path = default_config_path()?;
        Self::load_or_create_default_from_path(path)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn clipboard_message_template(&self) -> &str {
        &self.file.clipboard.message
    }

    pub fn editors(&self) -> &[String] {
        &self.file.editors
    }

    fn load_or_create_default_from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if !path.exists() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).with_context(|| {
                    format!(
                        "config ディレクトリを作成できませんでした: {}",
                        parent.display()
                    )
                })?;
            }
            fs::write(&path, DEFAULT_CONFIG_TOML).with_context(|| {
                format!("config ファイルを書き込めませんでした: {}", path.display())
            })?;
        }

        let raw = fs::read_to_string(&path).with_context(|| {
            format!("config ファイルを読み込めませんでした: {}", path.display())
        })?;
        let file = toml::from_str(&raw)
            .with_context(|| format!("config TOML を解釈できませんでした: {}", path.display()))?;

        Ok(Self { path, file })
    }
}

fn default_config_path() -> Result<PathBuf> {
    Ok(default_config_dir()?.join(CONFIG_FILE_NAME))
}

#[cfg(not(test))]
pub(crate) fn default_config_dir() -> Result<PathBuf> {
    if let Some(dir) = env_path(CONFIG_DIR_ENV) {
        return Ok(dir);
    }

    let base = dirs::config_local_dir()
        .unwrap_or(std::env::current_dir().context("current directory を取得できませんでした")?);
    Ok(base.join(CONFIG_DIR_NAME))
}

#[cfg(test)]
pub(crate) fn default_config_dir() -> Result<PathBuf> {
    if let Some(dir) = env_path(TEST_CONFIG_DIR_ENV) {
        return Ok(dir);
    }

    Ok(test_config_dir().join(CONFIG_DIR_NAME))
}

fn env_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

#[derive(Debug, Clone, Deserialize)]
struct ConfigFile {
    #[serde(default = "default_editors")]
    editors: Vec<String>,
    #[serde(default)]
    clipboard: ClipboardConfig,
}

impl Default for ConfigFile {
    fn default() -> Self {
        Self {
            editors: default_editors(),
            clipboard: ClipboardConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct ClipboardConfig {
    #[serde(default = "default_clipboard_message")]
    message: String,
}

impl Default for ClipboardConfig {
    fn default() -> Self {
        Self {
            message: DEFAULT_CLIPBOARD_MESSAGE.to_string(),
        }
    }
}

fn default_clipboard_message() -> String {
    DEFAULT_CLIPBOARD_MESSAGE.to_string()
}

fn default_editors() -> Vec<String> {
    DEFAULT_EDITORS
        .iter()
        .map(|editor| (*editor).to_string())
        .collect()
}

#[cfg(test)]
const TEST_CONFIG_DIR_ENV: &str = "GH_COMMIT_LEDGER_TEST_CONFIG_DIR";

#[cfg(test)]
fn test_config_dir() -> PathBuf {
    static TEST_CONFIG_DIR: OnceLock<PathBuf> = OnceLock::new();

    TEST_CONFIG_DIR
        .get_or_init(|| {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be after UNIX_EPOCH")
                .as_nanos();
            std::env::temp_dir().join(format!(
                "gh-commit-ledger-config-tests-{}-{unique}",
                std::process::id()
            ))
        })
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_editor_strings() -> Vec<String> {
        DEFAULT_EDITORS
            .iter()
            .map(|editor| (*editor).to_string())
            .collect()
    }

    #[test]
    fn missing_clipboard_message_uses_default() {
        let file: ConfigFile = toml::from_str("").unwrap();
        assert_eq!(file.clipboard.message, DEFAULT_CLIPBOARD_MESSAGE);
        assert_eq!(file.editors, default_editor_strings());
    }

    #[test]
    fn parses_clipboard_message_and_editors() {
        let file: ConfigFile = toml::from_str(
            r#"
editors = ["fresh", "code"]

[clipboard]
message = "yesterday={commits_yesterday}, total={total_commits}"
"#,
        )
        .unwrap();

        assert_eq!(
            file.clipboard.message,
            "yesterday={commits_yesterday}, total={total_commits}"
        );
        assert_eq!(file.editors, vec!["fresh", "code"]);
    }

    #[test]
    fn creates_default_config_when_missing() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "gh-commit-ledger-config-test-{}-{unique}",
            std::process::id()
        ));
        let path = dir.join("config.toml");

        let config = AppConfig::load_or_create_default_from_path(&path).unwrap();

        assert_eq!(config.path(), path.as_path());
        assert_eq!(
            config.clipboard_message_template(),
            DEFAULT_CLIPBOARD_MESSAGE
        );
        let generated = fs::read_to_string(&path).unwrap();
        assert!(generated.contains(r#"editors = ["fresh", "zed", "code", "edit", "nano", "vim"]"#));
        assert!(generated.contains("[clipboard]"));

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn default_loader_uses_unified_test_config_dir() {
        let config = AppConfig::load_or_create_default().unwrap();
        let expected_path = default_config_dir().unwrap().join(CONFIG_FILE_NAME);

        assert_eq!(config.path(), expected_path.as_path());
        assert!(config.path().starts_with(test_config_dir()));
    }
}
