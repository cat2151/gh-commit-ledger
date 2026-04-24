use std::ffi::OsStr;

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use tokio::process::Command;

#[derive(Debug, Clone)]
pub struct GhClient {
    pub max_parallelism: usize,
}

impl Default for GhClient {
    fn default() -> Self {
        Self { max_parallelism: 8 }
    }
}

impl GhClient {
    pub async fn list_public_repositories(&self) -> Result<Vec<RepoInfo>> {
        let login: Viewer = self.run_json(["api", "user"]).await?;
        let repos_args = vec![
            "repo".to_string(),
            "list".to_string(),
            login.login.clone(),
            "--limit".to_string(),
            "1000".to_string(),
            "--visibility".to_string(),
            "public".to_string(),
            "--source".to_string(),
            "--no-archived=false".to_string(),
            "--json".to_string(),
            "name,nameWithOwner,isArchived,isFork,defaultBranchRef".to_string(),
        ];
        let repos: Vec<RepoListItem> = self.run_json(repos_args).await?;

        let mut repos = repos
            .into_iter()
            .filter(|repo| !repo.is_fork)
            .map(RepoInfo::try_from)
            .collect::<Result<Vec<_>>>()?;

        repos.sort_by(|left, right| left.name_with_owner.cmp(&right.name_with_owner));
        Ok(repos)
    }

    pub async fn fetch_commit_window(
        &self,
        repo: &RepoInfo,
        current_cutoff: DateTime<Utc>,
        previous_cutoff: DateTime<Utc>,
    ) -> Result<CommitWindow> {
        if repo.default_branch.is_none() {
            return Ok(CommitWindow::default());
        }

        let query = r#"query RepoCommitWindow($owner: String!, $name: String!, $currentCutoff: GitTimestamp!, $previousCutoff: GitTimestamp!) {
  repository(owner: $owner, name: $name) {
    defaultBranchRef {
      target {
        __typename
        ... on Commit {
          current: history(until: $currentCutoff) {
            totalCount
          }
          previous: history(until: $previousCutoff) {
            totalCount
          }
        }
      }
    }
        }
}"#;

        let args = vec![
            "api".to_string(),
            "graphql".to_string(),
            "-f".to_string(),
            format!("query={query}"),
            "-f".to_string(),
            format!("owner={}", repo.owner),
            "-f".to_string(),
            format!("name={}", repo.name),
            "-f".to_string(),
            format!("currentCutoff={}", current_cutoff.to_rfc3339()),
            "-f".to_string(),
            format!("previousCutoff={}", previous_cutoff.to_rfc3339()),
        ];
        let response: GraphqlEnvelope<RepoCommitResponse> = self.run_json(args).await?;

        if let Some(errors) = response.errors {
            let combined = errors
                .into_iter()
                .map(|error| error.message)
                .collect::<Vec<_>>()
                .join("; ");
            bail!("GraphQL error for {}: {combined}", repo.name_with_owner);
        }

        let data = response
            .data
            .context("GraphQL response に data が含まれていませんでした")?;

        let target = data
            .repository
            .and_then(|repository| repository.default_branch_ref)
            .and_then(|branch| branch.target);

        let Some(target) = target else {
            return Ok(CommitWindow::default());
        };

        if target.type_name != "Commit" {
            return Ok(CommitWindow::default());
        }

        Ok(CommitWindow {
            current_total: target
                .current
                .map(|history| history.total_count)
                .unwrap_or(0),
            previous_total: target
                .previous
                .map(|history| history.total_count)
                .unwrap_or(0),
        })
    }

    async fn run_json<T, I, S>(&self, args: I) -> Result<T>
    where
        T: DeserializeOwned,
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let output = Command::new("gh")
            .args(args)
            .output()
            .await
            .context("gh コマンドの起動に失敗しました。GitHub CLI がインストールされているか確認してください")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let detail = if stderr.is_empty() { stdout } else { stderr };
            bail!("gh コマンドが失敗しました: {detail}");
        }

        serde_json::from_slice(&output.stdout).context("gh の JSON 出力を解釈できませんでした")
    }
}

#[derive(Debug, Clone)]
pub struct RepoInfo {
    pub owner: String,
    pub name: String,
    pub name_with_owner: String,
    pub is_archived: bool,
    pub default_branch: Option<String>,
}

impl TryFrom<RepoListItem> for RepoInfo {
    type Error = anyhow::Error;

    fn try_from(value: RepoListItem) -> Result<Self> {
        let (owner, _) = value
            .name_with_owner
            .split_once('/')
            .context("nameWithOwner の形式が不正です")?;

        Ok(Self {
            owner: owner.to_string(),
            name: value.name,
            name_with_owner: value.name_with_owner,
            is_archived: value.is_archived,
            default_branch: value.default_branch_ref.map(|branch| branch.name),
        })
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CommitWindow {
    pub current_total: u64,
    pub previous_total: u64,
}

#[derive(Debug, Deserialize)]
struct Viewer {
    login: String,
}

#[derive(Debug, Deserialize)]
struct RepoListItem {
    name: String,
    #[serde(rename = "nameWithOwner")]
    name_with_owner: String,
    #[serde(rename = "isArchived")]
    is_archived: bool,
    #[serde(rename = "isFork")]
    is_fork: bool,
    #[serde(rename = "defaultBranchRef")]
    default_branch_ref: Option<BranchRef>,
}

#[derive(Debug, Deserialize)]
struct BranchRef {
    name: String,
}

#[derive(Debug, Deserialize)]
struct GraphqlEnvelope<T> {
    data: Option<T>,
    errors: Option<Vec<GraphqlError>>,
}

#[derive(Debug, Deserialize)]
struct GraphqlError {
    message: String,
}

#[derive(Debug, Deserialize)]
struct RepoCommitResponse {
    repository: Option<RepositoryPayload>,
}

#[derive(Debug, Deserialize)]
struct RepositoryPayload {
    #[serde(rename = "defaultBranchRef")]
    default_branch_ref: Option<DefaultBranchPayload>,
}

#[derive(Debug, Deserialize)]
struct DefaultBranchPayload {
    target: Option<CommitTarget>,
}

#[derive(Debug, Deserialize)]
struct CommitTarget {
    #[serde(rename = "__typename")]
    type_name: String,
    current: Option<CommitHistory>,
    previous: Option<CommitHistory>,
}

#[derive(Debug, Deserialize)]
struct CommitHistory {
    #[serde(rename = "totalCount")]
    total_count: u64,
}
