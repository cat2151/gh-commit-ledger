use clap::{Parser, Subcommand};

pub(crate) enum StartupCommand {
    RunTui,
    Update,
    Hash,
    Check,
}

pub(crate) fn parse() -> StartupCommand {
    let cli = Cli::parse();
    cli.command.map(StartupCommand::from).unwrap_or_default()
}

impl Default for StartupCommand {
    fn default() -> Self {
        Self::RunTui
    }
}

impl From<CliCommand> for StartupCommand {
    fn from(command: CliCommand) -> Self {
        match command {
            CliCommand::Update => Self::Update,
            CliCommand::Hash => Self::Hash,
            CliCommand::Check => Self::Check,
        }
    }
}

#[derive(Debug, Parser)]
#[command(name = "gh-commit-ledger")]
#[command(about = "GitHub CLI から公開リポジトリの Commit Ledger を表示します")]
#[command(after_help = "引数なしで起動すると Commit Ledger TUI を表示します。")]
struct Cli {
    #[command(subcommand)]
    command: Option<CliCommand>,
}

#[derive(Debug, Subcommand)]
enum CliCommand {
    #[command(about = "GitHub から cargo install --force --git で自己更新します")]
    Update,
    #[command(about = "ビルド時に埋め込まれた commit hash を表示します")]
    Hash,
    #[command(about = "リモート main とビルド時 commit hash を比較します")]
    Check,
}
