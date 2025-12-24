use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::AppState;

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
                "ERROR" => Style::default().fg(Color::Red),
                "WARN" => Style::default().fg(Color::Yellow),
                "INFO" => Style::default().fg(Color::Cyan),
                "DEBUG" => Style::default().fg(Color::Gray),
                _ => Style::default().fg(Color::White),
            };
            
            Line::from(vec![
                Span::styled(format!("[{}] ", msg.level), level_style),
                Span::raw(&msg.message),
            ])
        })
        .collect();

    let block = Block::default()
        .title(" Logs ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let paragraph = Paragraph::new(log_text)
        .block(block)
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}
