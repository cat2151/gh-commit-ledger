use crate::config::AppConfig;
use crate::report::{ProgressUpdate, ReportOutcome};
use crate::ui;

pub(crate) struct App {
    screen: Screen,
    progress: ProgressUpdate,
    footer_message: String,
    cache_path: String,
    config_path: String,
    clipboard_message_template: String,
    editors: Vec<String>,
    loading: bool,
    should_quit: bool,
    tick: usize,
}

impl App {
    pub(crate) fn new(
        cache_path: String,
        config_path: String,
        clipboard_message_template: String,
        editors: Vec<String>,
    ) -> Self {
        Self {
            screen: Screen::Loading,
            progress: ProgressUpdate {
                note: "リポジトリ一覧を取得しています".to_string(),
                ..ProgressUpdate::default()
            },
            footer_message: "初回集計中です。e 設定編集  q 終了".to_string(),
            cache_path,
            config_path,
            clipboard_message_template,
            editors,
            loading: true,
            should_quit: false,
            tick: 0,
        }
    }

    pub(crate) fn advance_tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
    }

    pub(crate) fn apply_config(&mut self, config: &AppConfig) {
        self.config_path = config.path().display().to_string();
        self.clipboard_message_template = config.clipboard_message_template().to_string();
        self.editors = config.editors().to_vec();
    }

    pub(crate) fn begin_loading(&mut self, force_refresh: bool) {
        self.loading = true;
        self.screen = Screen::Loading;
        self.progress = ProgressUpdate {
            note: if force_refresh {
                "キャッシュを無視して再取得しています".to_string()
            } else {
                "レポートを集計しています".to_string()
            },
            ..ProgressUpdate::default()
        };
        self.footer_message = "再集計中です。e 設定編集  q 終了".to_string();
    }

    pub(crate) fn finish_ready(&mut self, outcome: ReportOutcome) {
        self.finish_loading(Screen::Ready(outcome));
    }

    pub(crate) fn finish_error(&mut self, error: String) {
        self.finish_loading(Screen::Error(error));
    }

    pub(crate) fn record_progress(&mut self, progress: ProgressUpdate) {
        self.progress = progress;
    }

    pub(crate) fn request_quit(&mut self) {
        self.should_quit = true;
    }

    pub(crate) fn set_footer_message(&mut self, message: impl Into<String>) {
        self.footer_message = message.into();
    }

    pub(crate) fn current_report(&self) -> Option<&ReportOutcome> {
        match &self.screen {
            Screen::Ready(report) => Some(report),
            _ => None,
        }
    }

    pub(crate) fn ui_state(&self) -> ui::UiState<'_> {
        match &self.screen {
            Screen::Loading => ui::UiState::Loading(&self.progress),
            Screen::Ready(outcome) => ui::UiState::Ready(outcome),
            Screen::Error(error) => ui::UiState::Error {
                error,
                progress: &self.progress,
            },
        }
    }

    pub(crate) fn footer_message(&self) -> &str {
        &self.footer_message
    }

    pub(crate) fn cache_path(&self) -> &str {
        &self.cache_path
    }

    pub(crate) fn config_path(&self) -> &str {
        &self.config_path
    }

    pub(crate) fn clipboard_message_template(&self) -> &str {
        &self.clipboard_message_template
    }

    pub(crate) fn editors(&self) -> &[String] {
        &self.editors
    }

    pub(crate) fn is_loading(&self) -> bool {
        self.loading
    }

    pub(crate) fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub(crate) fn tick(&self) -> usize {
        self.tick
    }

    fn finish_loading(&mut self, screen: Screen) {
        self.loading = false;
        self.screen = screen;
        self.footer_message = match &self.screen {
            Screen::Ready(_) => "c コピー  e 設定編集  r 再取得  q 終了".to_string(),
            Screen::Error(_) => "e 設定編集  r 再試行  q 終了".to_string(),
            Screen::Loading => "集計中です。e 設定編集".to_string(),
        };
    }
}

enum Screen {
    Loading,
    Ready(ReportOutcome),
    Error(String),
}
