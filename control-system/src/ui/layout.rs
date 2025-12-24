use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Tabs},
    Frame,
};
use tachyonfx::{Effect, EffectRenderer, Shader, Duration as FxDuration};

use crate::app::{AppState, Page};

use super::widgets::{
    activity_feed::render_activity_feed,
    clock::render_clock,
    github_overview::render_github_overview,
    help_overlay::render_help_overlay,
    log_viewer,
    repo_spotlight::render_repo_spotlight,
    status_bar::render_status_bar,
    system_stats::render_system_stats,
};

/// Main render function for the application
pub fn render_app(frame: &mut Frame, state: &AppState, effects: &mut Vec<Effect>) {
    let size = frame.area();

    // Main layout: header, content, status bar
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header with tabs
            Constraint::Min(10),    // Content area
            Constraint::Length(3),  // Status bar
        ])
        .split(size);

    // Render header with tabs
    render_header(frame, main_chunks[0], state);

    // Render current page content
    render_page_content(frame, main_chunks[1], state, effects);

    // Render status bar
    render_status_bar(frame, main_chunks[2], state);

    // Render help overlay if active
    if state.ui.show_help_overlay {
        render_help_overlay(frame, size);
    }

    // Apply active effects
    for effect in effects.iter_mut() {
        frame.render_effect(effect, size, FxDuration::from_millis(16));
    }

    // Clean up finished effects
    effects.retain(|e| !e.done());
}

/// Render the header with navigation tabs
fn render_header(frame: &mut Frame, area: Rect, state: &AppState) {
    let titles: Vec<Line> = vec!["1:Dashboard", "2:Repos", "3:Activity", "4:Settings"]
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let style = if i == state.ui.current_page.index() {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            Line::from(Span::styled(*t, style))
        })
        .collect();

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(Span::styled(
                    " control-system ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )),
        )
        .select(state.ui.current_page.index())
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .divider(Span::raw(" | "));

    frame.render_widget(tabs, area);
}

/// Render the content for the current page
fn render_page_content(frame: &mut Frame, area: Rect, state: &AppState, effects: &mut Vec<Effect>) {
    match state.ui.current_page {
        Page::Dashboard => render_dashboard(frame, area, state, effects),
        Page::Repositories => render_repositories_page(frame, area, state),
        Page::Activity => render_activity_page(frame, area, state, effects),
        Page::Settings => render_settings_page(frame, area, state),
    }
}

/// Render the dashboard page
fn render_dashboard(frame: &mut Frame, area: Rect, state: &AppState, _effects: &mut Vec<Effect>) {
    // Split into left and right columns
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(area);

    // Left column: GitHub overview and repo spotlight, with log viewer at bottom
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(35),  // GitHub overview
            Constraint::Percentage(45),  // Repo spotlight
            Constraint::Percentage(20),  // Log viewer
        ])
        .split(columns[0]);

    render_github_overview(frame, left_chunks[0], state);
    render_repo_spotlight(frame, left_chunks[1], state);
    log_viewer::render(frame, left_chunks[2], state);

    // Right column: Clock and system stats
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),  // Clock
            Constraint::Min(10),    // System stats
        ])
        .split(columns[1]);

    render_clock(frame, right_chunks[0], state);
    render_system_stats(frame, right_chunks[1], state);
}

/// Render the repositories page
fn render_repositories_page(frame: &mut Frame, area: Rect, state: &AppState) {
    use ratatui::widgets::{List, ListItem, Paragraph};
    
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Span::styled(
            " Repositories ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));

    if state.github.repos.is_empty() {
        let empty = Paragraph::new("No repositories loaded yet...")
            .block(block)
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(empty, area);
        return;
    }

    // Split into top starred and recently updated
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .margin(1)
        .split(area);

    // Draw outer block
    frame.render_widget(block, area);

    // Top starred repos
    let starred_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(" [*] Top Starred ", Style::default().fg(Color::Yellow)));

    let starred_repos = state.github.top_repos_by_stars(10);
    let starred_items: Vec<ListItem> = starred_repos
        .iter()
        .map(|repo| {
            let lang = repo.language.as_deref().unwrap_or("???");
            let line = Line::from(vec![
                Span::styled(
                    format!("*{:<4}", repo.stargazers_count),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw(" "),
                Span::styled(&repo.name, Style::default().fg(Color::Cyan)),
                Span::raw(" "),
                Span::styled(format!("[{}]", lang), Style::default().fg(Color::DarkGray)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let starred_list = List::new(starred_items).block(starred_block);
    frame.render_widget(starred_list, chunks[0]);

    // Recently updated repos
    let recent_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(" [>] Recently Updated ", Style::default().fg(Color::Green)));

    let recent_repos = state.github.recently_updated_repos(10);
    let recent_items: Vec<ListItem> = recent_repos
        .iter()
        .map(|repo| {
            let updated = repo
                .pushed_at
                .map(|t| crate::util::time::format_relative(t))
                .unwrap_or_else(|| "???".to_string());
            let line = Line::from(vec![
                Span::styled(&repo.name, Style::default().fg(Color::Cyan)),
                Span::raw(" "),
                Span::styled(updated, Style::default().fg(Color::DarkGray)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let recent_list = List::new(recent_items).block(recent_block);
    frame.render_widget(recent_list, chunks[1]);
}

/// Render the activity feed page
fn render_activity_page(frame: &mut Frame, area: Rect, state: &AppState, effects: &mut Vec<Effect>) {
    render_activity_feed(frame, area, state, effects);
}

/// Render the settings/help page
fn render_settings_page(frame: &mut Frame, area: Rect, state: &AppState) {
    use ratatui::widgets::Paragraph;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(12), // Keybinds
            Constraint::Length(8),  // Settings
            Constraint::Min(5),     // Rate limit info
        ])
        .margin(1)
        .split(area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Span::styled(
            " Settings & Help ",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ));
    frame.render_widget(block, area);

    // Keybinds section
    let keybinds_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" Keyboard Controls ");

    let keybind_text = crate::app::actions::keybind_help()
        .iter()
        .map(|(key, desc)| {
            Line::from(vec![
                Span::styled(format!("{:>8}", key), Style::default().fg(Color::Cyan)),
                Span::raw("  "),
                Span::styled(*desc, Style::default().fg(Color::White)),
            ])
        })
        .collect::<Vec<_>>();

    let keybinds = Paragraph::new(keybind_text).block(keybinds_block);
    frame.render_widget(keybinds, chunks[0]);

    // Settings section
    let settings_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" Animation Settings ");

    let motion_status = if state.fx.animations_paused {
        Span::styled("PAUSED", Style::default().fg(Color::Yellow))
    } else if state.fx.reduced_motion {
        Span::styled("REDUCED", Style::default().fg(Color::Yellow))
    } else {
        Span::styled("ENABLED", Style::default().fg(Color::Green))
    };

    let settings_text = vec![
        Line::from(vec![
            Span::raw("Animations: "),
            motion_status,
        ]),
        Line::from(vec![
            Span::raw("Press "),
            Span::styled("p", Style::default().fg(Color::Cyan)),
            Span::raw(" to toggle animation pause"),
        ]),
    ];

    let settings = Paragraph::new(settings_text).block(settings_block);
    frame.render_widget(settings, chunks[1]);

    // Rate limit info
    let rate_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" GitHub API Rate Limit ");

    let rate_limit = &state.github.rate_limit;
    let rate_color = if rate_limit.is_low() {
        Color::Red
    } else if rate_limit.remaining < rate_limit.limit / 2 {
        Color::Yellow
    } else {
        Color::Green
    };

    let reset_time = rate_limit
        .reset_at
        .map(|t| crate::util::time::format_relative(t))
        .unwrap_or_else(|| "???".to_string());

    let rate_text = vec![
        Line::from(vec![
            Span::raw("Remaining: "),
            Span::styled(
                format!("{}/{}", rate_limit.remaining, rate_limit.limit),
                Style::default().fg(rate_color),
            ),
        ]),
        Line::from(vec![
            Span::raw("Reset: "),
            Span::styled(reset_time, Style::default().fg(Color::DarkGray)),
        ]),
    ];

    let rate_info = Paragraph::new(rate_text).block(rate_block);
    frame.render_widget(rate_info, chunks[2]);
}
