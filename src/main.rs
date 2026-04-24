use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    gh_commit_ledger::run().await
}
