use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use ratatui_image::protocol::StatefulProtocol;
use std::sync::Mutex;

use crate::app::AppState;
use crate::spotify::state::PlayerState;

/// Area positions for clickable Spotify controls (set during render)
pub struct SpotifyClickAreas {
    pub prev_area: Option<Rect>,
    pub toggle_area: Option<Rect>,
    pub next_area: Option<Rect>,
}

static CLICK_AREAS: Mutex<Option<SpotifyClickAreas>> = Mutex::new(None);

/// Store click areas during render
pub fn store_click_areas(areas: SpotifyClickAreas) {
    if let Ok(mut guard) = CLICK_AREAS.lock() {
        *guard = Some(areas);
    }
}

/// Check if a click at (col, row) hit a Spotify control
pub fn check_spotify_click(col: u16, row: u16) -> Option<SpotifyAction> {
    if let Ok(guard) = CLICK_AREAS.lock() {
        if let Some(ref areas) = *guard {
            if let Some(ref area) = areas.prev_area {
                if col >= area.x
                    && col < area.x + area.width
                    && row >= area.y
                    && row < area.y + area.height
                {
                    return Some(SpotifyAction::Prev);
                }
            }
            if let Some(ref area) = areas.toggle_area {
                if col >= area.x
                    && col < area.x + area.width
                    && row >= area.y
                    && row < area.y + area.height
                {
                    return Some(SpotifyAction::Toggle);
                }
            }
            if let Some(ref area) = areas.next_area {
                if col >= area.x
                    && col < area.x + area.width
                    && row >= area.y
                    && row < area.y + area.height
                {
                    return Some(SpotifyAction::Next);
                }
            }
        }
    }
    None
}

#[derive(Debug, Clone, Copy)]
pub enum SpotifyAction {
    Prev,
    Toggle,
    Next,
}

/// Render the Spotify player widget
pub fn render_spotify_player(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    album_art_proto: &mut Option<StatefulProtocol>,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green))
        .title(Span::styled(
            " Spotify ",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ));

    if !state.spotify.connected {
        let msg = if let Some(ref err) = state.spotify.error {
            format!("Spotify: {}", err)
        } else {
            "Not configured (press S in Settings to set up)".to_string()
        };
        let paragraph = Paragraph::new(msg)
            .block(block)
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(paragraph, area);
        store_click_areas(SpotifyClickAreas {
            prev_area: None,
            toggle_area: None,
            next_area: None,
        });
        return;
    }

    let player = &state.spotify.player;

    if player.track_name.is_empty() {
        let paragraph = Paragraph::new("No track playing")
            .block(block)
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(paragraph, area);
        store_click_areas(SpotifyClickAreas {
            prev_area: None,
            toggle_area: None,
            next_area: None,
        });
        return;
    }

    // Inner layout
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Vertical: [top row: art + info] [controls] [progress bar]
    let main_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(4),    // Album art + track info (top)
            Constraint::Length(3), // Controls (tall for touch)
            Constraint::Length(1), // Progress bar (full width)
        ])
        .split(inner);

    // Top row: album art (left) + track info + queue (right)
    let has_art = album_art_proto.is_some();
    let top_chunks = if has_art && main_rows[0].width > 30 {
        let art_width = (main_rows[0].height * 2).min(main_rows[0].width / 3).max(8);
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(art_width), Constraint::Min(16)])
            .split(main_rows[0])
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(0), Constraint::Min(1)])
            .split(main_rows[0])
    };

    // LEFT: Album art with 1-cell padding
    if let Some(proto) = album_art_proto.as_mut() {
        let raw_art = top_chunks[0];
        if raw_art.width > 2 && raw_art.height > 1 {
            let padded = Rect {
                x: raw_art.x + 1,
                y: raw_art.y,
                width: raw_art.width.saturating_sub(2),
                height: raw_art.height.saturating_sub(1),
            };
            frame.render_stateful_widget(ratatui_image::StatefulImage::default(), padded, proto);
        }
    }

    // RIGHT: Track info (top) + queue (bottom)
    let raw_info = top_chunks[1];
    let right_area = if raw_info.width > 4 {
        Rect {
            x: raw_info.x + 2,
            y: raw_info.y,
            width: raw_info.width.saturating_sub(3),
            height: raw_info.height,
        }
    } else {
        raw_info
    };

    // Split right side into track details and queue
    let right_split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // Track details (title big + album + artist)
            Constraint::Min(2),    // Up next queue
        ])
        .split(right_area);

    // Track details
    let max_w = right_split[0].width as usize;
    let track_lines = vec![
        // Track name - BIGGEST: bold, underlined, on its own with space
        Line::from(Span::styled(
            truncate_str(&player.track_name, max_w.saturating_sub(1)),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )),
        // Blank line to give the title visual weight
        Line::from(""),
        // Album name - second biggest
        Line::from(Span::styled(
            truncate_str(&player.album_name, max_w.saturating_sub(1)),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        // Artist name
        Line::from(Span::styled(
            truncate_str(&player.artist_name, max_w.saturating_sub(1)),
            Style::default().fg(Color::Green),
        )),
    ];

    let track_info = Paragraph::new(track_lines);
    frame.render_widget(track_info, right_split[0]);

    // Up Next queue
    let queue_area = right_split[1];
    let max_qw = queue_area.width as usize;
    let mut queue_lines: Vec<Line> = vec![Line::from(Span::styled(
        "Up Next:",
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    ))];

    if player.queue.is_empty() {
        queue_lines.push(Line::from(Span::styled(
            "  nothing here yet.",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for (i, item) in player.queue.iter().enumerate() {
            let entry = format!(
                "  {}. {} - {}",
                i + 1,
                truncate_str(&item.name, max_qw.saturating_sub(8)),
                truncate_str(&item.artist, 20),
            );
            queue_lines.push(Line::from(Span::styled(
                truncate_str(&entry, max_qw.saturating_sub(1)),
                Style::default().fg(Color::DarkGray),
            )));
        }
    }

    frame.render_widget(Paragraph::new(queue_lines), queue_area);

    // Controls row (full width, tall for touch)
    render_controls(frame, main_rows[1], player);

    // Progress bar (full width at bottom)
    render_progress_bar(frame, main_rows[2], player);
}

/// Render the progress bar
fn render_progress_bar(frame: &mut Frame, area: Rect, player: &PlayerState) {
    let width = area.width as usize;
    if width < 10 {
        return;
    }

    let time_left = PlayerState::format_time(player.progress_ms);
    let time_right = PlayerState::format_time(player.duration_ms);
    let time_text_len = time_left.len() + time_right.len() + 2;

    let bar_width = width.saturating_sub(time_text_len + 2);
    let filled = (player.progress_fraction() * bar_width as f64) as usize;
    let empty = bar_width.saturating_sub(filled);

    let line = Line::from(vec![
        Span::styled(&time_left, Style::default().fg(Color::DarkGray)),
        Span::raw(" "),
        Span::styled("#".repeat(filled), Style::default().fg(Color::Green)),
        Span::styled("-".repeat(empty), Style::default().fg(Color::DarkGray)),
        Span::raw(" "),
        Span::styled(&time_right, Style::default().fg(Color::DarkGray)),
    ]);

    frame.render_widget(Paragraph::new(line), area);
}

/// Render playback controls (clickable, tall for touchscreen)
fn render_controls(frame: &mut Frame, area: Rect, player: &PlayerState) {
    let play_icon = if player.is_playing {
        "  ||  "
    } else {
        "  >>  "
    };

    let ctrl_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(5),  // left margin
            Constraint::Percentage(25), // prev
            Constraint::Percentage(5),  // gap
            Constraint::Percentage(30), // toggle
            Constraint::Percentage(5),  // gap
            Constraint::Percentage(25), // next
            Constraint::Percentage(5),  // right margin
        ])
        .split(area);

    store_click_areas(SpotifyClickAreas {
        prev_area: Some(ctrl_layout[1]),
        toggle_area: Some(ctrl_layout[3]),
        next_area: Some(ctrl_layout[5]),
    });

    let btn_style = Style::default().bg(Color::DarkGray);

    let prev = Paragraph::new(Line::from(Span::styled(
        " |<< ",
        btn_style.fg(Color::Cyan).add_modifier(Modifier::BOLD),
    )))
    .alignment(Alignment::Center);

    let toggle = Paragraph::new(Line::from(Span::styled(
        play_icon,
        btn_style.fg(Color::Green).add_modifier(Modifier::BOLD),
    )))
    .alignment(Alignment::Center);

    let next = Paragraph::new(Line::from(Span::styled(
        " >>| ",
        btn_style.fg(Color::Cyan).add_modifier(Modifier::BOLD),
    )))
    .alignment(Alignment::Center);

    frame.render_widget(prev, ctrl_layout[1]);
    frame.render_widget(toggle, ctrl_layout[3]);
    frame.render_widget(next, ctrl_layout[5]);
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len > 3 {
        format!("{}...", &s[..max_len - 3])
    } else {
        s[..max_len].to_string()
    }
}
