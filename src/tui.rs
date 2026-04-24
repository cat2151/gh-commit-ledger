use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use tokio::sync::mpsc::{self, UnboundedReceiver};

use crate::app::App;
use crate::cache::CacheStore;
use crate::clipboard::copy_report;
use crate::config::AppConfig;
use crate::config_editor::edit_config_file;
use crate::events::AppMessage;
use crate::gh::GhClient;
use crate::report_task::spawn_report_task;
use crate::terminal::{self, AppTerminal};
use crate::ui;

pub async fn run_tui() -> Result<()> {
    let mut terminal = terminal::setup()?;
    let result = run_app(&mut terminal).await;
    terminal::restore(&mut terminal)?;
    result
}

async fn run_app(terminal: &mut AppTerminal) -> Result<()> {
    let app_config = AppConfig::load_or_create_default()?;
    let mut cache = CacheStore::load_default()?;
    let client = GhClient::default();
    let (tx, mut rx) = mpsc::unbounded_channel();
    let mut app = App::new(
        cache.path().display().to_string(),
        app_config.path().display().to_string(),
        app_config.clipboard_message_template().to_string(),
        app_config.editors().to_vec(),
    );

    spawn_report_task(
        tx.clone(),
        client.clone(),
        cache.path().display().to_string(),
        false,
    );

    loop {
        app.advance_tick();
        handle_messages(&mut app, &mut cache, &mut rx)?;
        draw_app(terminal, &app)?;

        if app.should_quit() {
            break;
        }

        if event::poll(Duration::from_millis(120))? {
            let event = event::read()?;
            if let Event::Key(key) = event {
                if key.kind == KeyEventKind::Press {
                    handle_key(terminal, &mut app, &mut cache, &client, &tx, key.code);
                }
            }
        }
    }

    Ok(())
}

fn draw_app(terminal: &mut AppTerminal, app: &App) -> Result<()> {
    let clipboard_message = app.current_report().map(|report| {
        report
            .report
            .clipboard_message(app.clipboard_message_template())
    });

    terminal.draw(|frame| {
        ui::render(
            frame,
            app.ui_state(),
            clipboard_message.as_deref(),
            app.footer_message(),
            app.tick(),
            app.cache_path(),
            app.config_path(),
        )
    })?;

    Ok(())
}

fn handle_messages(
    app: &mut App,
    cache: &mut CacheStore,
    rx: &mut UnboundedReceiver<AppMessage>,
) -> Result<()> {
    while let Ok(message) = rx.try_recv() {
        match message {
            AppMessage::Progress(progress) => {
                app.record_progress(progress);
            }
            AppMessage::Finished(result) => match result {
                Ok(loaded) => {
                    cache.replace_with(loaded.cache);
                    cache.save()?;
                    app.finish_ready(loaded.outcome);
                }
                Err(error) => {
                    app.finish_error(error);
                }
            },
        }
    }

    Ok(())
}

fn handle_key(
    terminal: &mut AppTerminal,
    app: &mut App,
    cache: &mut CacheStore,
    client: &GhClient,
    tx: &mpsc::UnboundedSender<AppMessage>,
    code: KeyCode,
) {
    match code {
        KeyCode::Char('q') | KeyCode::Esc => app.request_quit(),
        KeyCode::Char('e') => edit_config(terminal, app),
        KeyCode::Char('r') => refresh_report(app, cache, client, tx),
        KeyCode::Char('c') => copy_current_report(app),
        _ => {}
    }
}

fn edit_config(terminal: &mut AppTerminal, app: &mut App) {
    let config_path = app.config_path().to_string();
    let editors = app.editors().to_vec();

    match edit_config_file(terminal, &config_path, &editors) {
        Ok(()) => match AppConfig::load_or_create_default() {
            Ok(app_config) => {
                app.apply_config(&app_config);
                app.set_footer_message("config を再読み込みしました");
            }
            Err(error) => {
                app.set_footer_message(format!("config の再読み込みに失敗しました: {error}"));
            }
        },
        Err(error) => {
            app.set_footer_message(format!("config 編集に失敗しました: {error}"));
        }
    }
}

fn refresh_report(
    app: &mut App,
    cache: &CacheStore,
    client: &GhClient,
    tx: &mpsc::UnboundedSender<AppMessage>,
) {
    if app.is_loading() {
        return;
    }

    app.begin_loading(true);
    spawn_report_task(
        tx.clone(),
        client.clone(),
        cache.path().display().to_string(),
        true,
    );
}

fn copy_current_report(app: &mut App) {
    if let Some(report) = app.current_report() {
        match copy_report(report, app.clipboard_message_template()) {
            Ok(()) => {
                app.set_footer_message("クリップボードにメッセージをコピーしました");
            }
            Err(error) => {
                app.set_footer_message(format!("コピーに失敗しました: {error}"));
            }
        }
    }
}
