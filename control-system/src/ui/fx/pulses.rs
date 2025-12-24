//! Pulse and animation effects for the UI
//!
//! These effects are simplified to work with tachyonfx's API

use ratatui::style::Color;
use tachyonfx::{fx, Effect, Duration};

/// Create a breathing pulse effect for focused elements
pub fn breathing_pulse() -> Effect {
    fx::ping_pong(fx::fade_to_fg(
        Color::Cyan,
        Duration::from_millis(1000),
    ))
}

/// Create a quick pulse for new items
pub fn new_item_pulse() -> Effect {
    fx::sequence(&[
        fx::fade_to_fg(Color::Green, Duration::from_millis(100)),
        fx::fade_from_fg(Color::Green, Duration::from_millis(300)),
    ])
}

/// Create an attention-grabbing pulse for errors or warnings
pub fn alert_pulse() -> Effect {
    fx::ping_pong(fx::fade_to_fg(
        Color::Red,
        Duration::from_millis(500),
    ))
}

/// Create a subtle glow effect
pub fn subtle_glow(color: Color) -> Effect {
    fx::ping_pong(fx::fade_to_fg(
        color,
        Duration::from_millis(2000),
    ))
}
