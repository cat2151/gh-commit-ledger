use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::config;

const CACHE_FILE_NAME: &str = "cache.json";
const CACHE_VERSION: u32 = 1;
const MAX_SNAPSHOTS_PER_REPO: usize = 16;

#[derive(Debug, Clone)]
pub struct CacheStore {
    path: PathBuf,
    file: CacheFile,
}

impl CacheStore {
    pub fn load_default() -> Result<Self> {
        let path = default_cache_path()?;
        Self::load_from_path(&path)
    }

    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let file = if path.exists() {
            let raw = fs::read_to_string(&path).with_context(|| {
                format!("cache ファイルを読み込めませんでした: {}", path.display())
            })?;
            serde_json::from_str(&raw)
                .with_context(|| format!("cache JSON を解釈できませんでした: {}", path.display()))?
        } else {
            CacheFile::default()
        };

        Ok(Self { path, file })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn get(&self, repo: &str, as_of: DateTime<Utc>) -> Option<u64> {
        let key = snapshot_key(as_of);
        self.file
            .repos
            .get(repo)
            .and_then(|repo_cache| repo_cache.snapshots.get(&key))
            .map(|snapshot| snapshot.total_count)
    }

    pub fn insert(&mut self, repo: impl Into<String>, as_of: DateTime<Utc>, total_count: u64) {
        let repo = repo.into();
        let entry = self.file.repos.entry(repo).or_default();
        entry.snapshots.insert(
            snapshot_key(as_of),
            CachedSnapshot {
                total_count,
                as_of,
                fetched_at: Utc::now(),
            },
        );

        while entry.snapshots.len() > MAX_SNAPSHOTS_PER_REPO {
            let oldest_key = entry.snapshots.keys().next().cloned();
            if let Some(oldest_key) = oldest_key {
                entry.snapshots.remove(&oldest_key);
            } else {
                break;
            }
        }
    }

    pub fn replace_with(&mut self, other: CacheStore) {
        self.file = other.file;
    }

    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "cache ディレクトリを作成できませんでした: {}",
                    parent.display()
                )
            })?;
        }

        let json =
            serde_json::to_string_pretty(&self.file).context("cache JSON の生成に失敗しました")?;
        fs::write(&self.path, json).with_context(|| {
            format!(
                "cache ファイルを書き込めませんでした: {}",
                self.path.display()
            )
        })
    }
}

fn default_cache_path() -> Result<PathBuf> {
    Ok(config::default_config_dir()?.join(CACHE_FILE_NAME))
}

fn snapshot_key(as_of: DateTime<Utc>) -> String {
    as_of.to_rfc3339()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheFile {
    #[serde(default = "default_cache_version")]
    version: u32,
    #[serde(default)]
    repos: BTreeMap<String, RepoCache>,
}

impl Default for CacheFile {
    fn default() -> Self {
        Self {
            version: CACHE_VERSION,
            repos: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct RepoCache {
    #[serde(default)]
    snapshots: BTreeMap<String, CachedSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedSnapshot {
    total_count: u64,
    as_of: DateTime<Utc>,
    fetched_at: DateTime<Utc>,
}

fn default_cache_version() -> u32 {
    CACHE_VERSION
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    #[test]
    fn round_trip_cache_lookup() {
        let path = PathBuf::from("cache.json");
        let mut cache = CacheStore {
            path,
            file: CacheFile::default(),
        };
        let as_of = Utc.with_ymd_and_hms(2026, 4, 23, 14, 59, 59).unwrap();
        cache.insert("owner/repo", as_of, 42);
        assert_eq!(cache.get("owner/repo", as_of), Some(42));
    }

    #[test]
    fn default_cache_path_uses_config_dir() {
        assert_eq!(
            default_cache_path().unwrap(),
            config::default_config_dir().unwrap().join(CACHE_FILE_NAME)
        );
    }
}
