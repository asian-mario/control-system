use ratatui::prelude::*;
use ratatui::widgets::{Paragraph, Wrap};

use crate::app::AppState;
use crate::ui::theme;

/// Renders the log messages widget
pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let messages = state.log_buffer.get_messages();

    let log_text: Vec<Line> = messages
        .iter()
        .rev()
        .take(area.height.saturating_sub(2) as usize)
        .rev()
        .map(|msg| {
            let level_style = match msg.level.as_str() {
                "ERROR" => Style::default().fg(theme::ERROR),
                "WARN" => Style::default().fg(theme::WARNING),
                "INFO" => Style::default().fg(theme::ACCENT),
                "DEBUG" => theme::secondary(),
                _ => theme::primary(),
            };

            Line::from(vec![
                Span::styled(format!("[{}] ", msg.level), level_style),
                Span::raw(&msg.message),
            ])
        })
        .collect();

    let block = theme::card("Recent Log");

    let paragraph = Paragraph::new(log_text)
        .block(block)
        .style(theme::secondary())
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}
