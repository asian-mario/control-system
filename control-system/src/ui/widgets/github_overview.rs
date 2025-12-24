use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::AppState;
use crate::util::format::format_count;

/// Render the GitHub overview widget
pub fn render_github_overview(frame: &mut Frame, area: Rect, state: &AppState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Span::styled(
            " GitHub Overview ",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ));

    if let Some(ref profile) = state.github.profile {
        let stats = &state.github.stats;
        
        let name_line = Line::from(vec![
            Span::styled(
                profile.name.as_deref().unwrap_or(&profile.login),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" (@{})", profile.login),
                Style::default().fg(Color::DarkGray),
            ),
        ]);

        let bio_line = if let Some(ref bio) = profile.bio {
            Line::from(Span::styled(
                crate::util::format::truncate_str(bio, 60),
                Style::default().fg(Color::Gray),
            ))
        } else {
            Line::from("")
        };

        let followers_line = Line::from(vec![
            Span::styled("[F] ", Style::default()),
            Span::styled(
                format_count(profile.followers as u64),
                Style::default().fg(Color::Cyan),
            ),
            Span::raw(" followers  "),
            Span::styled(
                format_count(profile.following as u64),
                Style::default().fg(Color::Cyan),
            ),
            Span::raw(" following"),
        ]);

        let stats_line = Line::from(vec![
            Span::styled("[*] ", Style::default().fg(Color::Yellow)),
            Span::styled(
                format_count(stats.total_stars as u64),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw("  "),
            Span::styled("[Y] ", Style::default()),
            Span::styled(
                format_count(stats.total_forks as u64),
                Style::default().fg(Color::Cyan),
            ),
            Span::raw("  "),
            Span::styled("[R] ", Style::default()),
            Span::styled(
                format!("{} repos", stats.total_repos),
                Style::default().fg(Color::Green),
            ),
        ]);

        let status_line = match &state.github.status {
            crate::github::FetchStatus::Fetching => Line::from(Span::styled(
                "[~] Refreshing...",
                Style::default().fg(Color::Yellow),
            )),
            crate::github::FetchStatus::Error(e) => Line::from(Span::styled(
                format!("[!] {}", crate::util::format::truncate_str(e, 40)),
                Style::default().fg(Color::Red),
            )),
            _ => Line::from(""),
        };

        let text = vec![
            Line::from(""),
            name_line,
            bio_line,
            Line::from(""),
            followers_line,
            stats_line,
            Line::from(""),
            status_line,
        ];

        let paragraph = Paragraph::new(text).block(block);
        frame.render_widget(paragraph, area);
    } else {
        // No profile loaded yet
        let loading_text = if state.github.status.is_fetching() {
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    "Loading GitHub profile...",
                    Style::default().fg(Color::Yellow),
                )),
            ]
        } else {
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    "No profile data",
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(Span::styled(
                    "Press 'r' to refresh",
                    Style::default().fg(Color::DarkGray),
                )),
            ]
        };

        let paragraph = Paragraph::new(loading_text).block(block);
        frame.render_widget(paragraph, area);
    }
}
