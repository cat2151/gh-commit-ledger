use anyhow::Result;

use crate::cli::StartupCommand;
use crate::self_update;
use crate::tui;

const BUILD_COMMIT_HASH: &str = env!("BUILD_COMMIT_HASH");
const REPO_OWNER: &str = "cat2151";
const REPO_NAME: &str = "gh-commit-ledger";
const MAIN_BRANCH: &str = "main";

pub(crate) async fn run(command: StartupCommand) -> Result<()> {
    match command {
        StartupCommand::RunTui => tui::run_tui().await,
        StartupCommand::Update => update(),
        StartupCommand::Hash => {
            println!("{BUILD_COMMIT_HASH}");
            Ok(())
        }
        StartupCommand::Check => check(),
    }
}

fn update() -> Result<()> {
    self_update::self_update(REPO_OWNER, REPO_NAME, &[])
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    println!("更新処理を別プロセスで開始しました");
    Ok(())
}

fn check() -> Result<()> {
    let result =
        self_update::check_remote_commit(REPO_OWNER, REPO_NAME, MAIN_BRANCH, BUILD_COMMIT_HASH)
            .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    println!("{result}");
    Ok(())
}
