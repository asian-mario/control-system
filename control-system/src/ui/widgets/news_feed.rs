use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use crate::news::NewsFeed;
use crate::util::format::truncate_str;

/// Render the news headlines widget
pub fn render_news_feed(frame: &mut Frame, area: Rect, news: &NewsFeed) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Span::styled(
            " MY News ",
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD),
        ));

    if news.is_loading && news.items.is_empty() {
        let loading = ratatui::widgets::Paragraph::new("Loading news...")
            .block(block)
            .style(Style::default().fg(Color::Yellow));
        frame.render_widget(loading, area);
        return;
    }

    if let Some(ref err) = news.error {
        if news.items.is_empty() {
            let error = ratatui::widgets::Paragraph::new(format!("Error: {}", err))
                .block(block)
                .style(Style::default().fg(Color::Red));
            frame.render_widget(error, area);
            return;
        }
    }

    if news.items.is_empty() {
        let empty = ratatui::widgets::Paragraph::new("No news available")
            .block(block)
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(empty, area);
        return;
    }

    // Calculate how many items we can show (3 lines per item)
    let available_lines = area.height.saturating_sub(2) as usize;
    let max_items = available_lines / 3;
    let display_width = area.width.saturating_sub(4) as usize;

    let items: Vec<ListItem> = news
        .items
        .iter()
        .take(max_items.max(1))
        .map(|item| {
            // Truncate title to fit on one line
            let title = truncate_str(&item.title, display_width);
            
            // Format time ago
            let time_ago = item.pub_date
                .map(|d| crate::util::time::format_relative(d))
                .unwrap_or_else(|| "recent".to_string());

            // Three lines per news item
            let lines = vec![
                // Line 1: Title
                Line::from(Span::styled(
                    title,
                    Style::default().fg(Color::White),
                )),
                // Line 2: Source and time
                Line::from(vec![
                    Span::styled(
                        format!("  {} ", &item.source),
                        Style::default().fg(Color::Cyan),
                    ),
                    Span::styled(
                        format!("- {}", time_ago),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]),
                // Line 3: Empty line for spacing
                Line::from(""),
            ];

            ListItem::new(lines)
        })
        .collect();

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}
