//! Shared visual language for the dashboard.
//!
//! Most of the interface stays quiet so selection and state remain easy to
//! scan on a small touchscreen.

use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, BorderType, Borders},
};

pub const BACKGROUND: Color = Color::Rgb(8, 10, 15);
pub const SURFACE: Color = Color::Rgb(21, 25, 34);
pub const SURFACE_SELECTED: Color = Color::Rgb(36, 43, 56);
pub const DIVIDER: Color = Color::Rgb(48, 55, 69);
pub const PRIMARY: Color = Color::Rgb(244, 246, 248);
pub const SECONDARY: Color = Color::Rgb(141, 151, 166);
pub const ACCENT: Color = Color::Rgb(90, 200, 250);
pub const SUCCESS: Color = Color::Rgb(48, 209, 88);
pub const WARNING: Color = Color::Rgb(255, 214, 10);
pub const ERROR: Color = Color::Rgb(255, 69, 58);

pub fn screen() -> Style {
    Style::default().bg(BACKGROUND).fg(PRIMARY)
}

pub fn surface() -> Style {
    Style::default().bg(SURFACE).fg(PRIMARY)
}

pub fn primary() -> Style {
    Style::default().fg(PRIMARY)
}

pub fn secondary() -> Style {
    Style::default().fg(SECONDARY)
}

pub fn accent() -> Style {
    Style::default().fg(ACCENT)
}

pub fn title() -> Style {
    primary().add_modifier(Modifier::BOLD)
}

/// A quiet, rounded card used by every top-level widget.
pub fn card(title_text: impl Into<String>) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(DIVIDER))
        .style(surface())
        .title(Span::styled(format!(" {} ", title_text.into()), title()))
}

pub fn selected() -> Style {
    Style::default()
        .fg(PRIMARY)
        .bg(SURFACE_SELECTED)
        .add_modifier(Modifier::BOLD)
}
