use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::AppState;
use crate::ui::theme;
use crate::util::format::format_bytes;

/// Render the system stats widget
pub fn render_system_stats(frame: &mut Frame, area: Rect, state: &AppState) {
    let block = theme::card("System");

    let sys = &state.system;

    // CPU usage bar
    let (cpu_filled, cpu_empty) = create_bar(sys.cpu_usage as f64, 100.0, 12);
    let cpu_color = usage_color(sys.cpu_usage as f64);

    // Memory usage bar
    let (mem_filled, mem_empty) = create_bar(sys.memory_percent as f64, 100.0, 12);
    let mem_color = usage_color(sys.memory_percent as f64);

    // Temperature display
    let (temp_str, temp_color) = if let Some(temp) = sys.cpu_temp {
        let color = if temp >= 80.0 {
            theme::ERROR
        } else if temp >= 60.0 {
            theme::WARNING
        } else {
            theme::SUCCESS
        };
        (format!("{:.0}C", temp), color)
    } else {
        ("N/A".to_string(), theme::SECONDARY)
    };

    let text = vec![
        Line::from(""),
        // CPU with temperature
        Line::from(vec![
            Span::styled(" CPU  ", theme::secondary()),
            Span::styled(cpu_filled, Style::default().fg(cpu_color)),
            Span::styled(cpu_empty, Style::default().fg(theme::DIVIDER)),
            Span::styled(
                format!(" {:5.1}%", sys.cpu_usage),
                Style::default().fg(cpu_color),
            ),
        ]),
        Line::from(vec![
            Span::styled("      Temperature  ", theme::secondary()),
            Span::styled(&temp_str, Style::default().fg(temp_color)),
        ]),
        Line::from(""),
        // Memory
        Line::from(vec![
            Span::styled(" MEM  ", theme::secondary()),
            Span::styled(mem_filled, Style::default().fg(mem_color)),
            Span::styled(mem_empty, Style::default().fg(theme::DIVIDER)),
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
                theme::secondary(),
            ),
        ]),
        Line::from(""),
        // Uptime
        Line::from(vec![
            Span::styled(" Uptime  ", theme::secondary()),
            Span::styled(sys.uptime_formatted(), theme::primary()),
        ]),
    ];

    let paragraph = Paragraph::new(text).block(block);
    frame.render_widget(paragraph, area);
}

/// Create a text-based progress bar
fn create_bar(value: f64, max: f64, width: usize) -> (String, String) {
    let ratio = (value / max).clamp(0.0, 1.0);
    let filled = (ratio * width as f64).round() as usize;
    let empty = width - filled;

    ("━".repeat(filled), "─".repeat(empty))
}

/// Get color based on usage percentage
fn usage_color(percentage: f64) -> Color {
    if percentage >= 90.0 {
        theme::ERROR
    } else if percentage >= 70.0 {
        theme::WARNING
    } else if percentage >= 50.0 {
        theme::WARNING
    } else {
        theme::SUCCESS
    }
}
