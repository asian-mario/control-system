use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem},
    Frame,
};

use crate::app::AppState;
use crate::ui::theme;

/// Render the repository spotlight widget
pub fn render_repo_spotlight(frame: &mut Frame, area: Rect, state: &AppState) {
    let block = theme::card("Top Repositories");

    if state.github.repos.is_empty() {
        let empty_text = if state.github.status.is_fetching() {
            "Loading repositories..."
        } else {
            "No repositories loaded"
        };

        let paragraph = ratatui::widgets::Paragraph::new(empty_text)
            .block(block)
            .style(theme::secondary());
        frame.render_widget(paragraph, area);
        return;
    }

    let top_repos = state.github.top_repos_by_stars(8);

    let items: Vec<ListItem> = top_repos
        .iter()
        .enumerate()
        .map(|(i, repo)| {
            let rank_style = match i {
                0 => Style::default()
                    .fg(theme::WARNING)
                    .add_modifier(Modifier::BOLD),
                1 | 2 => theme::accent(),
                _ => theme::secondary(),
            };

            let lang = repo.language.as_deref().unwrap_or("???");
            let lang_color = language_color(lang);

            let desc = repo
                .description
                .as_ref()
                .map(|d| crate::util::format::truncate_str(d, 40))
                .unwrap_or_default();

            let lines = vec![
                Line::from(vec![
                    Span::styled(format!("#{:<2}", i + 1), rank_style),
                    Span::styled(&repo.name, theme::title()),
                    Span::raw(" "),
                    Span::styled(format!("[{}]", lang), Style::default().fg(lang_color)),
                ]),
                Line::from(vec![
                    Span::raw("   "),
                    Span::styled("★", Style::default().fg(theme::WARNING)),
                    Span::styled(
                        format!("{:<5}", repo.stargazers_count),
                        Style::default().fg(theme::WARNING),
                    ),
                    Span::styled("Y", Style::default()),
                    Span::styled(format!("{:<4}", repo.forks_count), theme::accent()),
                    Span::styled(desc, theme::secondary()),
                ]),
            ];

            ListItem::new(lines)
        })
        .collect();

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}

/// Get a color for a programming language
fn language_color(lang: &str) -> Color {
    match lang.to_lowercase().as_str() {
        "rust" => Color::Rgb(222, 165, 132),
        "python" => Color::Rgb(53, 114, 165),
        "javascript" => Color::Rgb(241, 224, 90),
        "typescript" => Color::Rgb(49, 120, 198),
        "go" => Color::Rgb(0, 173, 216),
        "java" => Color::Rgb(176, 114, 25),
        "c++" | "cpp" => Color::Rgb(243, 75, 125),
        "c" => Color::Rgb(85, 85, 85),
        "c#" | "csharp" => Color::Rgb(104, 33, 122),
        "ruby" => Color::Rgb(112, 21, 22),
        "php" => Color::Rgb(79, 93, 149),
        "swift" => Color::Rgb(255, 172, 69),
        "kotlin" => Color::Rgb(169, 123, 255),
        "shell" | "bash" => Color::Rgb(137, 224, 81),
        "html" => Color::Rgb(227, 76, 38),
        "css" => Color::Rgb(86, 61, 124),
        "vue" => Color::Rgb(65, 184, 131),
        "react" => Color::Rgb(97, 218, 251),
        _ => Color::Gray,
    }
}
