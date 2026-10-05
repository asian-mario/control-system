use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::{app::AppState, ui::theme};

const WORDS: [&str; 7] = ["Are", "you", "lonely?", "I", "can", "fix", "that."];

/// Render a calm, audio-inspired ambient animation.
///
/// The wave and the seven-word sentence share a 14-second phase. Every wave
/// component completes a whole number of cycles and the text is invisible at
/// the phase boundary, so the loop joins without a visible jump.
pub fn render_visualizer(frame: &mut Frame, area: Rect, state: &AppState) {
    let block = theme::card("Visualizer");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = visualizer_frame(inner.width, inner.height, state.fx.visualizer_phase);
    frame.render_widget(Paragraph::new(lines).style(theme::surface()), inner);
}

fn visualizer_frame(width: u16, height: u16, phase: f32) -> Vec<Line<'static>> {
    if width == 0 || height == 0 {
        return Vec::new();
    }

    let phase = phase.rem_euclid(std::f32::consts::TAU);
    let center = (height.saturating_sub(1) as f32) / 2.0;
    let radius = center.max(1.0);
    let word = word_cue(phase);
    let letters = word.map(|(text, _)| tracked_letters(text, width as usize));
    let word_start = letters
        .as_ref()
        .map(|letters| (width as usize).saturating_sub(letters.len()) / 2)
        .unwrap_or(0);

    (0..height)
        .map(|row| {
            let distance = ((row as f32 - center).abs() / radius).clamp(0.0, 1.0);
            let spans = (0..width)
                .map(|column| {
                    let x = column as f32;
                    let position = if width > 1 {
                        x / (width - 1) as f32
                    } else {
                        0.5
                    };

                    // Several slow waves create organic movement without the
                    // unstable flicker of per-frame randomness.
                    let wave = (x * 0.43 + phase).sin() * 0.52
                        + (x * 0.19 - phase * 2.0).sin() * 0.31
                        + (x * 0.08 + phase * 3.0).cos() * 0.17;
                    let envelope = 0.42 + (position * std::f32::consts::PI).sin() * 0.58;
                    let energy = (wave.abs() * envelope - distance * 0.72).clamp(0.0, 1.0);

                    let (glyph, brightness) = if energy > 0.52 {
                        ("●", 1.0)
                    } else if energy > 0.27 {
                        ("•", 0.72)
                    } else if energy > 0.08 {
                        ("·", 0.42)
                    } else {
                        (" ", 0.0)
                    };

                    if row == height / 2 {
                        if let (Some((_, opacity)), Some(letters)) = (word, letters.as_ref()) {
                            if let Some(&letter) = (column as usize)
                                .checked_sub(word_start)
                                .and_then(|index| letters.get(index))
                            {
                                if letter != ' ' {
                                    let glint =
                                        0.85 + 0.15 * (phase * 7.0 - column as f32 * 0.45).sin();
                                    return Span::styled(
                                        letter.to_string(),
                                        Style::default().fg(word_color(opacity * glint)),
                                    );
                                }
                            }
                        }
                    }

                    Span::styled(glyph, Style::default().fg(visualizer_color(brightness)))
                })
                .collect::<Vec<_>>();

            Line::from(spans)
        })
        .collect()
}

/// A single word rises out of the dots, holds briefly, then dissolves.
fn word_cue(phase: f32) -> Option<(&'static str, f32)> {
    let position =
        phase.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU * WORDS.len() as f32;
    let index = position.floor() as usize % WORDS.len();
    let progress = position.fract();
    let opacity = smoothstep(0.05, 0.28, progress) * (1.0 - smoothstep(0.72, 0.95, progress));

    (opacity > 0.02).then_some((WORDS[index], opacity))
}

fn smoothstep(start: f32, end: f32, value: f32) -> f32 {
    let progress = ((value - start) / (end - start)).clamp(0.0, 1.0);
    progress * progress * (3.0 - 2.0 * progress)
}

fn tracked_letters(word: &str, width: usize) -> Vec<char> {
    let letters: Vec<char> = word.chars().collect();
    if letters.len() * 2 - 1 > width {
        return letters.into_iter().take(width).collect();
    }

    let mut tracked = Vec::with_capacity(letters.len() * 2 - 1);
    for (index, letter) in letters.into_iter().enumerate() {
        if index > 0 {
            tracked.push(' ');
        }
        tracked.push(letter);
    }
    tracked
}

fn word_color(opacity: f32) -> Color {
    let mix = opacity.clamp(0.0, 1.0);
    let blend = |low: u8, high: u8| low as f32 + (high as f32 - low as f32) * mix;

    Color::Rgb(
        blend(48, 205) as u8,
        blend(55, 230) as u8,
        blend(69, 245) as u8,
    )
}

fn visualizer_color(brightness: f32) -> Color {
    let mix = brightness.clamp(0.0, 1.0);
    let blend = |low: u8, high: u8| low as f32 + (high as f32 - low as f32) * mix;

    Color::Rgb(
        blend(48, 90) as u8,
        blend(55, 200) as u8,
        blend(69, 250) as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::{visualizer_frame, word_cue, WORDS};

    #[test]
    fn frame_matches_requested_dimensions() {
        let frame = visualizer_frame(24, 5, 1.25);
        assert_eq!(frame.len(), 5);
        assert!(frame.iter().all(|line| line.width() == 24));
    }

    #[test]
    fn empty_area_produces_no_lines() {
        assert!(visualizer_frame(0, 5, 0.0).is_empty());
        assert!(visualizer_frame(12, 0, 0.0).is_empty());
    }

    #[test]
    fn loop_joins_without_a_frame_change() {
        assert_eq!(
            visualizer_frame(40, 7, 0.0),
            visualizer_frame(40, 7, std::f32::consts::TAU)
        );
    }

    #[test]
    fn words_appear_one_at_a_time_in_order() {
        assert!(word_cue(0.0).is_none());
        for (index, expected) in WORDS.iter().enumerate() {
            let phase = (index as f32 + 0.5) * std::f32::consts::TAU / WORDS.len() as f32;
            assert_eq!(word_cue(phase).map(|(word, _)| word), Some(*expected));
        }
        assert!(word_cue(std::f32::consts::TAU).is_none());
    }
}
