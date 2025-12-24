use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};
use tachyonfx::Effect;

use crate::app::AppState;
use crate::util::time::format_relative;

/// Render the activity feed widget
pub fn render_activity_feed(frame: &mut Frame, area: Rect, state: &AppState, _effects: &mut Vec<Effect>) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Span::styled(
            " Activity Feed ",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ));

    if state.github.events.is_empty() {
        let empty_text = if state.github.status.is_fetching() {
            "Loading activity..."
        } else {
            "No recent activity"
        };

        let paragraph = ratatui::widgets::Paragraph::new(empty_text)
            .block(block)
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(paragraph, area);
        return;
    }

    let items: Vec<ListItem> = state
        .github
        .events
        .iter()
        .take(20)
        .map(|event| {
            let icon = event.event_type.icon();
            let desc = event.event_type.description();
            let time = format_relative(event.created_at);

            // Extract repo name without username prefix
            let repo_short = event
                .repo_name
                .split('/')
                .last()
                .unwrap_or(&event.repo_name);

            let style = if event.is_new {
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let line = Line::from(vec![
                Span::styled(format!("{} ", icon), style),
                Span::styled(desc, Style::default().fg(Color::White)),
                Span::raw(" "),
                Span::styled(repo_short, Style::default().fg(Color::Cyan)),
                Span::raw(" "),
                Span::styled(time, Style::default().fg(Color::DarkGray)),
                if event.is_new {
                    Span::styled(" NEW", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
                } else {
                    Span::raw("")
                },
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}
