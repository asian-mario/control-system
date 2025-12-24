use chrono::Local;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::AppState;

/// Render the clock widget
pub fn render_clock(frame: &mut Frame, area: Rect, state: &AppState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Span::styled(
            " Clock ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));

    let now = Local::now();
    
    // Use pulse value for subtle animation
    let pulse = state.fx.pulse_value();
    let time_color = if state.fx.should_animate() {
        // Subtle color shift based on pulse
        let brightness = (200.0 + (pulse * 55.0)) as u8;
        Color::Rgb(brightness, brightness, brightness)
    } else {
        Color::White
    };

    let time_str = now.format("%H:%M:%S").to_string();
    let date_str = now.format("%A").to_string();
    let full_date = now.format("%B %d, %Y").to_string();

    let text = vec![
        Line::from(Span::styled(
            &time_str,
            Style::default()
                .fg(time_color)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            &date_str,
            Style::default().fg(Color::Cyan),
        )),
        Line::from(Span::styled(
            &full_date,
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let paragraph = Paragraph::new(text)
        .block(block)
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}
