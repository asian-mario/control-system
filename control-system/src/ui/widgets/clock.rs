use chrono::Local;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::AppState;
use crate::ui::theme;

/// Render the clock widget
pub fn render_clock(frame: &mut Frame, area: Rect, state: &AppState) {
    let block = theme::card("Today");

    let now = Local::now();

    // Use pulse value for subtle animation
    let pulse = state.fx.pulse_value();
    let time_color = if state.fx.should_animate() {
        // Subtle color shift based on pulse
        let brightness = (200.0 + (pulse * 55.0)) as u8;
        Color::Rgb(brightness, brightness, brightness)
    } else {
        theme::PRIMARY
    };

    let time_str = now.format("%H:%M:%S").to_string();
    let date_str = now.format("%A").to_string();
    let full_date = now.format("%B %d, %Y").to_string();

    let text = vec![
        Line::from(Span::styled(
            &time_str,
            Style::default().fg(time_color).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(&date_str, theme::accent())),
        Line::from(Span::styled(&full_date, theme::secondary())),
    ];

    let paragraph = Paragraph::new(text)
        .block(block)
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}
