use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Tabs},
    Frame,
};
use tachyonfx::{Duration as FxDuration, Effect, EffectRenderer};

use ratatui_image::protocol::StatefulProtocol;

use crate::app::{AppState, Page};

use super::widgets::{
    activity_feed::render_activity_feed, clock::render_clock,
    github_overview::render_github_overview, help_overlay::render_help_overlay, log_viewer,
    news_feed::render_news_feed, spotify_player::render_spotify_player,
    status_bar::render_status_bar, system_stats::render_system_stats,
};

/// Main render function for the application
pub fn render_app(
    frame: &mut Frame,
    state: &AppState,
    effects: &mut Vec<Effect>,
    album_art_proto: &mut Option<StatefulProtocol>,
) {
    let size = frame.area();

    // Main layout: header, content, status bar
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header with tabs
            Constraint::Min(10),   // Content area
            Constraint::Length(3), // Status bar
        ])
        .split(size);

    // Render header with tabs
    render_header(frame, main_chunks[0], state);

    // Render current page content
    render_page_content(frame, main_chunks[1], state, effects, album_art_proto);

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
    let titles: Vec<Line> = vec![
        "1:Dashboard",
        "2:Repos",
        "3:Activity",
        "4:Spotify",
        "5:Settings",
    ]
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
fn render_page_content(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    effects: &mut Vec<Effect>,
    album_art_proto: &mut Option<StatefulProtocol>,
) {
    match state.ui.current_page {
        Page::Dashboard => render_dashboard(frame, area, state, effects, album_art_proto),
        Page::Repositories => render_repositories_page(frame, area, state),
        Page::Activity => render_activity_page(frame, area, state, effects),
        Page::Spotify => render_spotify_page(frame, area, state, album_art_proto),
        Page::Settings => render_settings_page(frame, area, state),
    }
}

/// Render the dashboard page
fn render_dashboard(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    effects: &mut Vec<Effect>,
    album_art_proto: &mut Option<StatefulProtocol>,
) {
    // Split into left and right columns
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(area);

    // Left column: top row (overview + activity), spotify player, log viewer
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(30), // GitHub overview + Activity
            Constraint::Percentage(50), // Spotify player
            Constraint::Percentage(20), // Log viewer
        ])
        .split(columns[0]);

    // Split top row into GitHub Overview (left) and Activity Feed (right)
    let top_row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(left_chunks[0]);

    render_github_overview(frame, top_row[0], state);
    render_activity_feed(frame, top_row[1], state, effects);
    render_spotify_player(frame, left_chunks[1], state, album_art_proto);
    log_viewer::render(frame, left_chunks[2], state);

    // Right column: Clock, news, and system stats
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),  // Clock
            Constraint::Min(8),     // News feed
            Constraint::Length(12), // System stats
        ])
        .split(columns[1]);

    render_clock(frame, right_chunks[0], state);
    render_news_feed(frame, right_chunks[1], &state.news);
    render_system_stats(frame, right_chunks[2], state);
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
        .title(Span::styled(
            " [*] Top Starred ",
            Style::default().fg(Color::Yellow),
        ));

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
        .title(Span::styled(
            " [>] Recently Updated ",
            Style::default().fg(Color::Green),
        ));

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
fn render_activity_page(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    effects: &mut Vec<Effect>,
) {
    render_activity_feed(frame, area, state, effects);
}

/// Render the full Spotify page (TUI Spotify-like experience)
fn render_spotify_page(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    album_art_proto: &mut Option<StatefulProtocol>,
) {
    use ratatui::widgets::Paragraph;

    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green))
        .title(Span::styled(
            " Spotify ",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ));

    if !state.spotify.connected {
        let msg = if let Some(ref err) = state.spotify.error {
            format!("Not connected: {}", err)
        } else {
            "Spotify not configured. Press S in Settings to set up.".to_string()
        };
        let p = Paragraph::new(msg)
            .block(outer)
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(p, area);
        return;
    }

    let player = &state.spotify.player;
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    if player.track_name.is_empty() {
        let p = Paragraph::new("No track playing").style(Style::default().fg(Color::DarkGray));
        frame.render_widget(p, inner);
        return;
    }

    // Main layout: left sidebar (queue) | center (now playing)
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(inner);

    // LEFT: Up Next queue panel
    let queue_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(
            " Up Next ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));
    let queue_inner = queue_block.inner(columns[0]);
    frame.render_widget(queue_block, columns[0]);

    let max_qw = queue_inner.width as usize;
    let mut queue_lines: Vec<Line> = Vec::new();

    if player.queue.is_empty() {
        queue_lines.push(Line::from(""));
        queue_lines.push(Line::from(Span::styled(
            "  nothing here yet.",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for (i, item) in player.queue.iter().enumerate() {
            let num_span = Span::styled(
                format!(" {:>2}. ", i + 1),
                Style::default().fg(Color::DarkGray),
            );
            let name_span = Span::styled(
                truncate_str_local(&item.name, max_qw.saturating_sub(6)),
                Style::default().fg(Color::White),
            );
            queue_lines.push(Line::from(vec![num_span, name_span]));
            queue_lines.push(Line::from(Span::styled(
                format!(
                    "      {}",
                    truncate_str_local(&item.artist, max_qw.saturating_sub(7))
                ),
                Style::default().fg(Color::DarkGray),
            )));
        }
    }
    frame.render_widget(Paragraph::new(queue_lines), queue_inner);

    // RIGHT: Now Playing
    let right_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(8),    // Album art + track info
            Constraint::Length(1), // Progress bar
            Constraint::Length(4), // Controls (with top padding)
        ])
        .split(columns[1]);

    // Now Playing area: art (left) + info (right)
    let np_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(
            " Now Playing ",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ));
    let np_inner = np_block.inner(right_rows[0]);
    frame.render_widget(np_block, right_rows[0]);

    let has_art = album_art_proto.is_some();
    let np_cols = if has_art && np_inner.width > 30 {
        let art_w = (np_inner.height * 2).min(np_inner.width / 2).max(8);
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(art_w), Constraint::Min(16)])
            .split(np_inner)
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(0), Constraint::Min(1)])
            .split(np_inner)
    };

    // Album art
    if let Some(proto) = album_art_proto.as_mut() {
        let raw_art = np_cols[0];
        if raw_art.width > 2 && raw_art.height > 1 {
            let padded = Rect {
                x: raw_art.x + 1,
                y: raw_art.y,
                width: raw_art.width.saturating_sub(2),
                height: raw_art.height.saturating_sub(1),
            };
            frame.render_stateful_widget(ratatui_image::StatefulImage::default(), padded, proto);
        }
    }

    // Track info
    let info_area = Rect {
        x: np_cols[1].x + 2,
        y: np_cols[1].y + 1,
        width: np_cols[1].width.saturating_sub(3),
        height: np_cols[1].height.saturating_sub(1),
    };
    let max_w = info_area.width as usize;

    let mut info_lines = vec![
        Line::from(Span::styled(
            truncate_str_local(&player.track_name, max_w.saturating_sub(1)),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )),
        Line::from(""),
        Line::from(Span::styled(
            truncate_str_local(&player.album_name, max_w.saturating_sub(1)),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            truncate_str_local(&player.artist_name, max_w.saturating_sub(1)),
            Style::default().fg(Color::Green),
        )),
    ];

    // Fill remaining height with blank
    let used = 4;
    for _ in used..info_area.height as usize {
        info_lines.push(Line::from(""));
    }

    frame.render_widget(Paragraph::new(info_lines), info_area);

    // Progress bar (full width)
    let prog_area = right_rows[1];
    let prog_w = prog_area.width as usize;
    if prog_w >= 10 {
        let time_left = crate::spotify::state::PlayerState::format_time(player.progress_ms);
        let time_right = crate::spotify::state::PlayerState::format_time(player.duration_ms);
        let time_text_len = time_left.len() + time_right.len() + 2;
        let bar_w = prog_w.saturating_sub(time_text_len + 2);
        let filled = (player.progress_fraction() * bar_w as f64) as usize;
        let empty = bar_w.saturating_sub(filled);
        let prog_line = Line::from(vec![
            Span::styled(&time_left, Style::default().fg(Color::DarkGray)),
            Span::raw(" "),
            Span::styled("#".repeat(filled), Style::default().fg(Color::Green)),
            Span::styled("-".repeat(empty), Style::default().fg(Color::DarkGray)),
            Span::raw(" "),
            Span::styled(&time_right, Style::default().fg(Color::DarkGray)),
        ]);
        frame.render_widget(Paragraph::new(prog_line), prog_area);
    }

    // Controls (with top padding)
    let ctrl_outer = right_rows[2];
    let ctrl_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(ctrl_outer);
    let ctrl_area = ctrl_rows[1];

    let play_icon = if player.is_playing {
        "  | |  "
    } else {
        "  > > >"
    };
    let ctrl_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(5),
            Constraint::Percentage(25),
            Constraint::Percentage(5),
            Constraint::Percentage(30),
            Constraint::Percentage(5),
            Constraint::Percentage(25),
            Constraint::Percentage(5),
        ])
        .split(ctrl_area);

    // Register click areas so mouse clicks work on this page
    use super::widgets::spotify_player::{store_click_areas, SpotifyClickAreas};
    store_click_areas(SpotifyClickAreas {
        prev_area: Some(ctrl_cols[1]),
        toggle_area: Some(ctrl_cols[3]),
        next_area: Some(ctrl_cols[5]),
    });

    let btn = Style::default().bg(Color::DarkGray);
    let prev_w = Paragraph::new(Line::from(Span::styled(
        " |<< ",
        btn.fg(Color::Cyan).add_modifier(Modifier::BOLD),
    )))
    .alignment(Alignment::Center);
    let toggle_w = Paragraph::new(Line::from(Span::styled(
        play_icon,
        btn.fg(Color::Green).add_modifier(Modifier::BOLD),
    )))
    .alignment(Alignment::Center);
    let next_w = Paragraph::new(Line::from(Span::styled(
        " >>| ",
        btn.fg(Color::Cyan).add_modifier(Modifier::BOLD),
    )))
    .alignment(Alignment::Center);

    frame.render_widget(prev_w, ctrl_cols[1]);
    frame.render_widget(toggle_w, ctrl_cols[3]);
    frame.render_widget(next_w, ctrl_cols[5]);
}

fn truncate_str_local(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len > 3 {
        format!("{}...", &s[..max_len - 3])
    } else {
        s[..max_len].to_string()
    }
}

/// Render the settings/help page
fn render_settings_page(frame: &mut Frame, area: Rect, state: &AppState) {
    use ratatui::widgets::Paragraph;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(12), // Keybinds
            Constraint::Length(8),  // Animation settings
            Constraint::Length(6),  // Spotify settings
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
        Line::from(vec![Span::raw("Animations: "), motion_status]),
        Line::from(vec![
            Span::raw("Press "),
            Span::styled("p", Style::default().fg(Color::Cyan)),
            Span::raw(" to toggle animation pause"),
        ]),
    ];

    let settings = Paragraph::new(settings_text).block(settings_block);
    frame.render_widget(settings, chunks[1]);

    // Spotify settings section
    let spotify_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" Spotify ");

    let spotify_status = if state.spotify.connected {
        Span::styled("CONNECTED", Style::default().fg(Color::Green))
    } else if state.spotify.error.is_some() {
        Span::styled("ERROR", Style::default().fg(Color::Red))
    } else {
        Span::styled("NOT CONFIGURED", Style::default().fg(Color::DarkGray))
    };

    let spotify_text = vec![
        Line::from(vec![Span::raw("Status: "), spotify_status]),
        Line::from(vec![
            Span::raw("Press "),
            Span::styled("S", Style::default().fg(Color::Cyan)),
            Span::raw(" to reset Spotify (clears tokens, restart to re-setup)"),
        ]),
    ];

    let spotify_settings = Paragraph::new(spotify_text).block(spotify_block);
    frame.render_widget(spotify_settings, chunks[2]);

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
    frame.render_widget(rate_info, chunks[3]);
}
