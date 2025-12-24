use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::AppState;
use crate::util::format::format_bytes;

/// Render the system stats widget
pub fn render_system_stats(frame: &mut Frame, area: Rect, state: &AppState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Span::styled(
            " System ",
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        ));

    let sys = &state.system;

    // CPU usage bar
    let cpu_bar = create_bar(sys.cpu_usage as f64, 100.0, 15);
    let cpu_color = usage_color(sys.cpu_usage as f64);

    // Memory usage bar
    let mem_bar = create_bar(sys.memory_percent as f64, 100.0, 15);
    let mem_color = usage_color(sys.memory_percent as f64);

    let text = vec![
        Line::from(""),
        // CPU
        Line::from(vec![
            Span::styled(" CPU ", Style::default().fg(Color::White)),
            Span::styled(cpu_bar, Style::default().fg(cpu_color)),
            Span::styled(
                format!(" {:5.1}%", sys.cpu_usage),
                Style::default().fg(cpu_color),
            ),
        ]),
        Line::from(""),
        // Memory
        Line::from(vec![
            Span::styled(" MEM ", Style::default().fg(Color::White)),
            Span::styled(mem_bar, Style::default().fg(mem_color)),
            Span::styled(
                format!(" {:5.1}%", sys.memory_percent),
                Style::default().fg(mem_color),
            ),
        ]),
        Line::from(vec![
            Span::raw("      "),
            Span::styled(
                format!(
                    "{} / {}",
                    format_bytes(sys.memory_used),
                    format_bytes(sys.memory_total)
                ),
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        Line::from(""),
        // Uptime
        Line::from(vec![
            Span::styled(" Uptime: ", Style::default().fg(Color::White)),
            Span::styled(
                sys.uptime_formatted(),
                Style::default().fg(Color::Green),
            ),
        ]),
        // Host info
        Line::from(vec![
            Span::styled(" Host: ", Style::default().fg(Color::White)),
            Span::styled(
                &sys.hostname,
                Style::default().fg(Color::Cyan),
            ),
        ]),
    ];

    let paragraph = Paragraph::new(text).block(block);
    frame.render_widget(paragraph, area);
}

/// Create a text-based progress bar
fn create_bar(value: f64, max: f64, width: usize) -> String {
    let ratio = (value / max).clamp(0.0, 1.0);
    let filled = (ratio * width as f64).round() as usize;
    let empty = width - filled;

    format!("[{}{}]", "#".repeat(filled), "-".repeat(empty))
}

/// Get color based on usage percentage
fn usage_color(percentage: f64) -> Color {
    if percentage >= 90.0 {
        Color::Red
    } else if percentage >= 70.0 {
        Color::Yellow
    } else if percentage >= 50.0 {
        Color::LightYellow
    } else {
        Color::Green
    }
}
