use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Wrap};

use crate::report::{APP_TITLE, ProgressUpdate, ReportOutcome, format_number};

const MONOKAI_BG: Color = Color::Rgb(39, 40, 34);
const MONOKAI_PANEL_BG: Color = Color::Rgb(47, 48, 42);
const MONOKAI_FG: Color = Color::Rgb(248, 248, 242);
const MONOKAI_COMMENT: Color = Color::Rgb(117, 113, 94);
const MONOKAI_PINK: Color = Color::Rgb(249, 38, 114);
const MONOKAI_ORANGE: Color = Color::Rgb(253, 151, 31);
const MONOKAI_YELLOW: Color = Color::Rgb(230, 219, 116);
const MONOKAI_GREEN: Color = Color::Rgb(166, 226, 46);
const MONOKAI_BLUE: Color = Color::Rgb(102, 217, 239);
const MONOKAI_PURPLE: Color = Color::Rgb(174, 129, 255);
const MONOKAI_LIGHT_GRAY: Color = Color::Rgb(220, 220, 215);

pub enum UiState<'a> {
    Loading(&'a ProgressUpdate),
    Ready(&'a ReportOutcome),
    Error {
        error: &'a str,
        progress: &'a ProgressUpdate,
    },
}

pub fn render(
    frame: &mut Frame<'_>,
    state: UiState<'_>,
    clipboard_message: Option<&str>,
    footer_message: &str,
    tick: usize,
    cache_path: &str,
    config_path: &str,
) {
    frame.render_widget(
        Block::default().style(Style::new().bg(MONOKAI_BG).fg(MONOKAI_FG)),
        frame.area(),
    );

    match state {
        UiState::Loading(progress) => {
            let chunks = Layout::vertical([
                Constraint::Length(8),
                Constraint::Min(8),
                Constraint::Length(5),
            ])
            .split(frame.area());
            render_loading(frame, chunks[0], chunks[1], progress, tick);
            render_footer(frame, chunks[2], footer_message, cache_path, config_path);
        }
        UiState::Ready(outcome) => {
            let chunks = Layout::vertical([
                Constraint::Length(8),
                Constraint::Length(5),
                Constraint::Min(8),
                Constraint::Length(5),
            ])
            .split(frame.area());
            render_ready(
                frame,
                chunks[0],
                chunks[1],
                chunks[2],
                outcome,
                clipboard_message.unwrap_or(""),
            );
            render_footer(frame, chunks[3], footer_message, cache_path, config_path);
        }
        UiState::Error { error, progress } => {
            let chunks = Layout::vertical([
                Constraint::Length(8),
                Constraint::Min(8),
                Constraint::Length(5),
            ])
            .split(frame.area());
            render_error(frame, chunks[0], chunks[1], error, progress);
            render_footer(frame, chunks[2], footer_message, cache_path, config_path);
        }
    }
}

fn render_loading(
    frame: &mut Frame<'_>,
    summary_area: Rect,
    body_area: Rect,
    progress: &ProgressUpdate,
    tick: usize,
) {
    let summary = vec![
        Line::from(vec![
            Span::styled(
                APP_TITLE,
                Style::new().fg(MONOKAI_BLUE).add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                "集計中",
                Style::new().fg(MONOKAI_YELLOW).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("昨日 23:59:59", Style::new().fg(MONOKAI_PURPLE)),
            Span::raw(" 時点の総 commit 数と差分を計算します"),
        ]),
        Line::raw(""),
        Line::from(vec![
            label("進捗: "),
            value(format_number(progress.processed as u64), MONOKAI_GREEN),
            Span::styled(" / ", Style::new().fg(MONOKAI_COMMENT)),
            value(format_number(progress.total as u64), MONOKAI_BLUE),
            Span::raw("  "),
            label("cache "),
            value(format_number(progress.cached as u64), MONOKAI_PURPLE),
            Span::raw("  "),
            label("fetch "),
            value(format_number(progress.fetched as u64), MONOKAI_ORANGE),
        ]),
        Line::from(vec![Span::styled(
            progress.note.clone(),
            Style::new().fg(MONOKAI_YELLOW),
        )]),
    ];

    frame.render_widget(
        Paragraph::new(summary)
            .block(panel(" Summary ", MONOKAI_PINK))
            .style(panel_text_style())
            .wrap(Wrap { trim: true }),
        summary_area,
    );

    let spinner = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"][tick % 10];
    let body = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                spinner,
                Style::new().fg(MONOKAI_PINK).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(
                progress
                    .current_repo
                    .as_deref()
                    .unwrap_or("リポジトリ一覧を確認しています"),
                Style::new().fg(MONOKAI_LIGHT_GRAY),
            ),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("初回実行", Style::new().fg(MONOKAI_ORANGE)),
            Span::raw("は全 public repository を走査するため、少し時間がかかります。"),
        ]),
    ])
    .block(panel(" Loading ", MONOKAI_ORANGE))
    .style(panel_text_style())
    .alignment(Alignment::Center)
    .wrap(Wrap { trim: true });

    frame.render_widget(body, body_area);
}

fn render_ready(
    frame: &mut Frame<'_>,
    summary_area: Rect,
    message_area: Rect,
    body_area: Rect,
    outcome: &ReportOutcome,
    clipboard_message: &str,
) {
    let report = &outcome.report;
    let summary = vec![
        Line::from(vec![Span::styled(
            report.report_date.format("%Y-%m-%d").to_string(),
            Style::new().fg(MONOKAI_PINK).add_modifier(Modifier::BOLD),
        )]),
        Line::raw(""),
        Line::from(vec![
            label("昨日のcommit数: "),
            value(format_number(report.commits_yesterday), MONOKAI_GREEN),
        ]),
        Line::from(vec![
            label("総commit数: "),
            value(format_number(report.total_commits), MONOKAI_PURPLE),
        ]),
        Line::from(vec![
            label("対象repo: "),
            value(format_number(outcome.total_repos as u64), MONOKAI_BLUE),
            Span::raw("  "),
            label("cache: "),
            value(format_number(outcome.cached_repos as u64), MONOKAI_PURPLE),
            Span::raw("  "),
            label("fetch: "),
            value(format_number(outcome.fetched_repos as u64), MONOKAI_ORANGE),
        ]),
    ];

    frame.render_widget(
        Paragraph::new(summary)
            .block(panel(" Summary ", MONOKAI_PINK))
            .style(panel_text_style()),
        summary_area,
    );

    frame.render_widget(
        Paragraph::new(clipboard_message)
            .block(panel(" Clipboard Message ", MONOKAI_ORANGE))
            .style(Style::new().fg(MONOKAI_FG).bg(MONOKAI_PANEL_BG))
            .wrap(Wrap { trim: true }),
        message_area,
    );

    let visible_rows = body_area.height.saturating_sub(3) as usize;
    let rows = if report.active_repos.is_empty() {
        vec![
            Row::new(vec![
                Cell::from("動きがあったリポジトリはありません")
                    .style(Style::new().fg(MONOKAI_COMMENT)),
                Cell::from(""),
                Cell::from(""),
            ])
            .style(panel_text_style()),
        ]
    } else {
        report
            .active_repos
            .iter()
            .enumerate()
            .take(visible_rows.max(1))
            .map(|(index, repo)| {
                let repo_name = if repo.is_archived {
                    Line::from(vec![
                        Span::styled(
                            repo.name_with_owner.clone(),
                            Style::new().fg(MONOKAI_LIGHT_GRAY),
                        ),
                        Span::styled(" [archived]", Style::new().fg(MONOKAI_ORANGE)),
                    ])
                } else {
                    Line::from(vec![Span::styled(
                        repo.name_with_owner.clone(),
                        Style::new().fg(MONOKAI_LIGHT_GRAY),
                    )])
                };
                Row::new(vec![
                    Cell::from(repo_name),
                    Cell::from(format!("+{}", format_number(repo.delta)))
                        .style(Style::new().fg(MONOKAI_GREEN).add_modifier(Modifier::BOLD)),
                    Cell::from(format_number(repo.total)).style(Style::new().fg(MONOKAI_PURPLE)),
                ])
                .style(Style::new().bg(if index % 2 == 0 {
                    MONOKAI_PANEL_BG
                } else {
                    MONOKAI_BG
                }))
            })
            .collect()
    };

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(70),
            Constraint::Length(12),
            Constraint::Length(14),
        ],
    )
    .header(
        Row::new(vec![
            table_header("Repository", MONOKAI_BLUE),
            table_header("Yesterday", MONOKAI_GREEN),
            table_header("Total", MONOKAI_PURPLE),
        ])
        .style(panel_text_style()),
    )
    .block(panel(" Active Repositories ", MONOKAI_YELLOW))
    .column_spacing(2);

    frame.render_widget(table, body_area);
}

fn render_error(
    frame: &mut Frame<'_>,
    summary_area: Rect,
    body_area: Rect,
    error: &str,
    progress: &ProgressUpdate,
) {
    let summary = vec![
        Line::from(vec![
            Span::styled(
                APP_TITLE,
                Style::new().fg(MONOKAI_PINK).add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                "取得失敗",
                Style::new().fg(MONOKAI_ORANGE).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::raw(""),
        Line::from(vec![
            label("進捗: "),
            value(format_number(progress.processed as u64), MONOKAI_GREEN),
            Span::styled(" / ", Style::new().fg(MONOKAI_COMMENT)),
            value(format_number(progress.total as u64), MONOKAI_BLUE),
        ]),
        Line::from(vec![Span::styled(
            progress.note.clone(),
            Style::new().fg(MONOKAI_YELLOW),
        )]),
    ];

    frame.render_widget(
        Paragraph::new(summary)
            .block(panel(" Summary ", MONOKAI_PINK))
            .style(panel_text_style())
            .wrap(Wrap { trim: true }),
        summary_area,
    );

    let popup = centered_rect(80, 60, body_area);
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(error)
            .block(panel(" Error ", MONOKAI_PINK))
            .style(Style::new().fg(MONOKAI_PINK).bg(MONOKAI_PANEL_BG))
            .wrap(Wrap { trim: true }),
        popup,
    );
}

fn render_footer(
    frame: &mut Frame<'_>,
    area: Rect,
    footer_message: &str,
    cache_path: &str,
    config_path: &str,
) {
    let footer = Paragraph::new(vec![
        Line::from(vec![Span::styled(
            footer_message.to_string(),
            Style::new().fg(MONOKAI_GREEN),
        )]),
        Line::from(vec![
            light_label("cache: "),
            Span::styled(cache_path.to_string(), Style::new().fg(MONOKAI_LIGHT_GRAY)),
        ]),
        Line::from(vec![
            light_label("config: "),
            Span::styled(config_path.to_string(), Style::new().fg(MONOKAI_LIGHT_GRAY)),
        ]),
    ])
    .block(panel(" Controls ", MONOKAI_GREEN))
    .style(panel_text_style())
    .wrap(Wrap { trim: true });

    frame.render_widget(footer, area);
}

fn panel(title: &'static str, accent: Color) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(Style::new().fg(MONOKAI_FG).bg(MONOKAI_PANEL_BG))
        .border_style(Style::new().fg(accent).bg(MONOKAI_PANEL_BG))
        .title_style(
            Style::new()
                .fg(accent)
                .bg(MONOKAI_PANEL_BG)
                .add_modifier(Modifier::BOLD),
        )
}

fn panel_text_style() -> Style {
    Style::new().fg(MONOKAI_FG).bg(MONOKAI_PANEL_BG)
}

fn label(text: &'static str) -> Span<'static> {
    Span::styled(text, Style::new().fg(MONOKAI_COMMENT))
}

fn light_label(text: &'static str) -> Span<'static> {
    Span::styled(text, Style::new().fg(MONOKAI_LIGHT_GRAY))
}

fn value(text: String, color: Color) -> Span<'static> {
    Span::styled(text, Style::new().fg(color).add_modifier(Modifier::BOLD))
}

fn table_header(text: &'static str, color: Color) -> Cell<'static> {
    Cell::from(text).style(
        Style::new()
            .fg(color)
            .bg(MONOKAI_PANEL_BG)
            .add_modifier(Modifier::BOLD)
            .add_modifier(Modifier::UNDERLINED),
    )
}

fn centered_rect(horizontal: u16, vertical: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([
        Constraint::Percentage((100 - vertical) / 2),
        Constraint::Percentage(vertical),
        Constraint::Percentage((100 - vertical) / 2),
    ])
    .split(area);
    Layout::horizontal([
        Constraint::Percentage((100 - horizontal) / 2),
        Constraint::Percentage(horizontal),
        Constraint::Percentage((100 - horizontal) / 2),
    ])
    .split(vertical[1])[1]
}
