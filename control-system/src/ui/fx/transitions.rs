//! Page transition effects
//!
//! Simplified effects using only fade transitions that work with tachyonfx

use ratatui::layout::Rect;
use tachyonfx::{fx, Duration, Effect};

use crate::ui::theme;

/// Create a fade-in effect for page transitions
pub fn fade_in() -> Effect {
    fx::fade_from_fg(theme::BACKGROUND, Duration::from_millis(220))
}

/// Create a fade-out effect for page transitions
pub fn fade_out() -> Effect {
    fx::fade_to_fg(theme::BACKGROUND, Duration::from_millis(160))
}

/// Get a page transition effect based on direction
pub fn get_page_transition(_from_page: usize, _to_page: usize, _area: Rect) -> Effect {
    // Use fade transition as a simple, compatible effect
    fade_in()
}

/// Create a combined fade transition
pub fn combined_transition(_area: Rect, _forward: bool) -> Effect {
    fade_in()
}
