use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Duration, Local, LocalResult, NaiveDate, TimeZone, Utc};
use tokio::sync::Semaphore;
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::JoinSet;

use crate::cache::CacheStore;
use crate::events::AppMessage;
use crate::gh::{GhClient, RepoInfo};

#[cfg(test)]
use crate::config::DEFAULT_CLIPBOARD_MESSAGE;

pub const APP_TITLE: &str = "Commit Ledger";

#[derive(Debug, Clone, Default)]
pub struct ProgressUpdate {
    pub processed: usize,
    pub total: usize,
    pub cached: usize,
    pub fetched: usize,
    pub current_repo: Option<String>,
    pub note: String,
}

#[derive(Debug, Clone)]
pub struct ReportOutcome {
    pub report: DailyReport,
    pub total_repos: usize,
    pub cached_repos: usize,
    pub fetched_repos: usize,
}

#[derive(Debug, Clone)]
pub struct DailyReport {
    pub report_date: NaiveDate,
    pub total_commits: u64,
    pub commits_yesterday: u64,
    pub active_repos: Vec<RepoDelta>,
}

impl DailyReport {
    pub fn clipboard_message(&self, message_template: &str) -> String {
        message_template
            .replace(
                "{commits_yesterday}",
                &format_number(self.commits_yesterday),
            )
            .replace("{total_commits}", &format_number(self.total_commits))
    }
}

#[derive(Debug, Clone)]
pub struct RepoDelta {
    pub name_with_owner: String,
    pub delta: u64,
    pub total: u64,
    pub is_archived: bool,
}

pub async fn generate_report(
    client: &GhClient,
    cache: &mut CacheStore,
    force_refresh: bool,
    tx: &UnboundedSender<AppMessage>,
) -> Result<ReportOutcome> {
    let cutoffs = ReportCutoffs::for_now(Local::now())?;
    send_progress(
        tx,
        ProgressUpdate {
            note: "GitHub CLI でログインユーザーを確認しています (gh api user)".to_string(),
            ..ProgressUpdate::default()
        },
    );

    let login = client.viewer_login().await?;
    send_progress(
        tx,
        ProgressUpdate {
            note: format!(
                "GitHub CLI で {login} の公開リポジトリ一覧を取得しています (gh repo list)"
            ),
            ..ProgressUpdate::default()
        },
    );

    let repos = client.list_public_repositories_for_login(&login).await?;
    let total = repos.len();
    let mut processed = 0usize;
    let mut cached_repos = 0usize;
    let mut fetched_repos = 0usize;
    let mut snapshots = Vec::with_capacity(total);

    send_progress(
        tx,
        ProgressUpdate {
            total,
            note: format!(
                "{} リポジトリの cache を照合しています",
                format_number(total as u64)
            ),
            ..ProgressUpdate::default()
        },
    );

    let semaphore = Arc::new(Semaphore::new(client.max_parallelism));
    let mut join_set = JoinSet::new();

    for repo in repos {
        if !force_refresh {
            let current = cache.get(&repo.name_with_owner, cutoffs.current_as_of);
            let previous = cache.get(&repo.name_with_owner, cutoffs.previous_as_of);

            if let (Some(current_total), Some(previous_total)) = (current, previous) {
                snapshots.push(RepoSnapshot {
                    repo,
                    current_total,
                    previous_total,
                });
                processed += 1;
                cached_repos += 1;
                send_progress(
                    tx,
                    ProgressUpdate {
                        processed,
                        total,
                        cached: cached_repos,
                        fetched: fetched_repos,
                        note: "cache にある snapshot を利用しました".to_string(),
                        ..ProgressUpdate::default()
                    },
                );
                continue;
            }
        }

        let repo_name = repo.name_with_owner.clone();
        let current_as_of = cutoffs.current_as_of;
        let previous_as_of = cutoffs.previous_as_of;
        let semaphore = semaphore.clone();
        let client = client.clone();

        join_set.spawn(async move {
            let _permit = semaphore
                .acquire_owned()
                .await
                .context("fetch の同時実行制御に失敗しました")?;
            let window = client
                .fetch_commit_window(&repo, current_as_of, previous_as_of)
                .await
                .with_context(|| format!("{repo_name} の commit 数取得に失敗しました"))?;
            Ok::<_, anyhow::Error>((repo, window))
        });
    }

    let pending_fetches = total.saturating_sub(processed);
    if pending_fetches > 0 {
        send_progress(
            tx,
            ProgressUpdate {
                processed,
                total,
                cached: cached_repos,
                fetched: fetched_repos,
                note: format!(
                    "cache にない {} リポジトリを GitHub から取得しています (gh api graphql)",
                    format_number(pending_fetches as u64)
                ),
                ..ProgressUpdate::default()
            },
        );
    }

    while let Some(result) = join_set.join_next().await {
        let (repo, window) = result.context("集計タスクの join に失敗しました")??;
        cache.insert(
            &repo.name_with_owner,
            cutoffs.current_as_of,
            window.current_total,
        );
        cache.insert(
            &repo.name_with_owner,
            cutoffs.previous_as_of,
            window.previous_total,
        );
        snapshots.push(RepoSnapshot {
            repo: repo.clone(),
            current_total: window.current_total,
            previous_total: window.previous_total,
        });

        processed += 1;
        fetched_repos += 1;
        send_progress(
            tx,
            ProgressUpdate {
                processed,
                total,
                cached: cached_repos,
                fetched: fetched_repos,
                current_repo: Some(repo.name_with_owner),
                note: "GitHub から取得しました (gh api graphql)".to_string(),
            },
        );
    }

    let report = build_daily_report(cutoffs.report_date, snapshots);

    Ok(ReportOutcome {
        report,
        total_repos: total,
        cached_repos,
        fetched_repos,
    })
}

fn build_daily_report(report_date: NaiveDate, mut snapshots: Vec<RepoSnapshot>) -> DailyReport {
    snapshots.sort_by(|left, right| left.repo.name_with_owner.cmp(&right.repo.name_with_owner));

    let total_commits = snapshots
        .iter()
        .map(|snapshot| snapshot.current_total)
        .sum();
    let commits_yesterday = snapshots
        .iter()
        .map(|snapshot| {
            snapshot
                .current_total
                .saturating_sub(snapshot.previous_total)
        })
        .sum();

    let mut active_repos = snapshots
        .into_iter()
        .filter_map(|snapshot| {
            let delta = snapshot
                .current_total
                .saturating_sub(snapshot.previous_total);
            (delta > 0).then_some(RepoDelta {
                name_with_owner: snapshot.repo.name_with_owner,
                delta,
                total: snapshot.current_total,
                is_archived: snapshot.repo.is_archived,
            })
        })
        .collect::<Vec<_>>();

    active_repos.sort_by(|left, right| {
        right
            .delta
            .cmp(&left.delta)
            .then_with(|| left.name_with_owner.cmp(&right.name_with_owner))
    });

    DailyReport {
        report_date,
        total_commits,
        commits_yesterday,
        active_repos,
    }
}

fn send_progress(tx: &UnboundedSender<AppMessage>, progress: ProgressUpdate) {
    let _ = tx.send(AppMessage::Progress(progress));
}

#[derive(Debug, Clone)]
struct ReportCutoffs {
    report_date: NaiveDate,
    current_as_of: DateTime<Utc>,
    previous_as_of: DateTime<Utc>,
}

impl ReportCutoffs {
    fn for_now(now: DateTime<Local>) -> Result<Self> {
        let today = now.date_naive();
        let report_date = today
            .checked_sub_signed(Duration::days(1))
            .context("昨日の日付を計算できませんでした")?;
        let previous_date = today
            .checked_sub_signed(Duration::days(2))
            .context("一昨日の日付を計算できませんでした")?;

        Ok(Self {
            report_date,
            current_as_of: local_end_of_day(report_date)?.with_timezone(&Utc),
            previous_as_of: local_end_of_day(previous_date)?.with_timezone(&Utc),
        })
    }
}

fn local_end_of_day(date: NaiveDate) -> Result<DateTime<Local>> {
    let naive = date
        .and_hms_opt(23, 59, 59)
        .ok_or_else(|| anyhow!("不正な時刻です: {date} 23:59:59"))?;

    match Local.from_local_datetime(&naive) {
        LocalResult::Single(value) => Ok(value),
        LocalResult::Ambiguous(value, _) => Ok(value),
        LocalResult::None => Err(anyhow!("ローカル時刻へ変換できませんでした: {naive}")),
    }
}

pub fn format_number(value: u64) -> String {
    let raw = value.to_string();
    let mut formatted = String::with_capacity(raw.len() + raw.len() / 3);
    for (index, ch) in raw.chars().rev().enumerate() {
        if index != 0 && index % 3 == 0 {
            formatted.push(',');
        }
        formatted.push(ch);
    }
    formatted.chars().rev().collect()
}

#[derive(Debug, Clone)]
struct RepoSnapshot {
    repo: RepoInfo,
    current_total: u64,
    previous_total: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_number_inserts_commas() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(12), "12");
        assert_eq!(format_number(1234), "1,234");
        assert_eq!(format_number(9_876_543), "9,876,543");
    }

    #[test]
    fn clipboard_message_uses_template_only() {
        let report = DailyReport {
            report_date: NaiveDate::from_ymd_opt(2026, 4, 23).unwrap(),
            total_commits: 1234,
            commits_yesterday: 7,
            active_repos: vec![RepoDelta {
                name_with_owner: "owner/repo".to_string(),
                delta: 7,
                total: 1234,
                is_archived: false,
            }],
        };

        let message = report.clipboard_message(DEFAULT_CLIPBOARD_MESSAGE);

        assert_eq!(
            message,
            "昨日は 7 件の commit をしました。\nこれまでの総 commit 数は 1,234 件です。"
        );
        assert!(!message.contains(APP_TITLE));
        assert!(!message.contains("owner/repo"));
    }

    #[test]
    fn clipboard_message_replaces_count_placeholders() {
        let report = DailyReport {
            report_date: NaiveDate::from_ymd_opt(2026, 4, 23).unwrap(),
            total_commits: 1234,
            commits_yesterday: 7,
            active_repos: vec![],
        };

        assert_eq!(
            report.clipboard_message("昨日={commits_yesterday} 総数={total_commits}"),
            "昨日=7 総数=1,234"
        );
    }
}
