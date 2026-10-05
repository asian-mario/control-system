//! Pulse and animation effects for the UI
//!
//! These effects are simplified to work with tachyonfx's API

use ratatui::style::Color;
use tachyonfx::{fx, Duration, Effect};

use crate::ui::theme;

/// Create a breathing pulse effect for focused elements
pub fn breathing_pulse() -> Effect {
    fx::ping_pong(fx::fade_to_fg(theme::ACCENT, Duration::from_millis(1400)))
}

/// Create a quick pulse for new items
pub fn new_item_pulse() -> Effect {
    fx::sequence(&[
        fx::fade_to_fg(theme::SUCCESS, Duration::from_millis(120)),
        fx::fade_from_fg(theme::SUCCESS, Duration::from_millis(320)),
    ])
}

/// Create an attention-grabbing pulse for errors or warnings
pub fn alert_pulse() -> Effect {
    fx::ping_pong(fx::fade_to_fg(theme::ERROR, Duration::from_millis(500)))
}

/// Create a subtle glow effect
pub fn subtle_glow(color: Color) -> Effect {
    fx::ping_pong(fx::fade_to_fg(color, Duration::from_millis(2000)))
}
