use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Clear, Paragraph},
    Frame,
};

use crate::app::actions::keybind_help;
use crate::ui::theme;

/// Render the help overlay
pub fn render_help_overlay(frame: &mut Frame, area: Rect) {
    // Calculate centered popup area
    let popup_area = centered_rect(50, 60, area);

    // Clear the area behind the popup
    frame.render_widget(Clear, popup_area);

    let block = theme::card("Keyboard Controls");

    let keybinds = keybind_help();
    let mut lines: Vec<Line> = vec![Line::from("")];

    for (key, desc) in keybinds {
        lines.push(Line::from(vec![
            Span::styled(format!("{:>10}", key), Style::default().fg(theme::ACCENT)),
            Span::raw("  "),
            Span::styled(desc.to_string(), theme::primary()),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Press ? or h to close",
        theme::secondary(),
    )));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Left);

    frame.render_widget(paragraph, popup_area);
}

/// Create a centered rect with percentage of parent
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
