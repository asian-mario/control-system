use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::AppState;
use crate::github::FetchStatus;

/// Render the status bar at the bottom
pub fn render_status_bar(frame: &mut Frame, area: Rect, state: &AppState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    // Status message
    let status_msg = state.status_message();
    let status_color = match &state.github.status {
        FetchStatus::Fetching => Color::Yellow,
        FetchStatus::Error(_) => Color::Red,
        FetchStatus::Success => Color::Green,
        FetchStatus::Idle => Color::DarkGray,
    };

    // Animation status indicator
    let anim_indicator = if state.fx.animations_paused {
        Span::styled(" [PAUSED] ", Style::default().fg(Color::Yellow))
    } else if state.fx.should_animate() {
        // Animated spinner effect using frame count
        let spinner_frames = ['|', '/', '-', '\\'];
        let frame_idx = (state.fx.frame_count / 3) as usize % spinner_frames.len();
        Span::styled(
            format!(" {} ", spinner_frames[frame_idx]),
            Style::default().fg(Color::Cyan),
        )
    } else {
        Span::raw(" ")
    };

    // Rate limit indicator
    let rate_limit = &state.github.rate_limit;
    let rate_color = if rate_limit.is_low() {
        Color::Red
    } else {
        Color::DarkGray
    };

    let rate_indicator = Span::styled(
        format!(" API: {}/{} ", rate_limit.remaining, rate_limit.limit),
        Style::default().fg(rate_color),
    );

    // Page indicator
    let page_indicator = Span::styled(
        format!(
            " [{}/4] {} ",
            state.ui.current_page.index() + 1,
            state.ui.current_page.title()
        ),
        Style::default().fg(Color::Cyan),
    );

    // Help hint
    let help_hint = Span::styled(
        " Press ? for help ",
        Style::default().fg(Color::DarkGray),
    );

    let line = Line::from(vec![
        anim_indicator,
        Span::raw("│"),
        Span::styled(format!(" {} ", status_msg), Style::default().fg(status_color)),
        Span::raw("│"),
        rate_indicator,
        Span::raw("│"),
        page_indicator,
        Span::raw("│"),
        help_hint,
    ]);

    let paragraph = Paragraph::new(line).block(block);
    frame.render_widget(paragraph, area);
}
