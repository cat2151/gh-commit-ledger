mod app;
mod cache;
mod cli;
mod clipboard;
mod config;
mod config_editor;
mod events;
mod gh;
mod report;
mod report_task;
mod self_update;
mod startup;
mod terminal;
mod tui;
mod ui;

use anyhow::Result;

pub async fn run() -> Result<()> {
    let command = cli::parse();
    startup::run(command).await
}
