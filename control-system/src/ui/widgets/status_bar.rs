use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::AppState;
use crate::github::FetchStatus;
use crate::ui::theme;

/// Render the quiet, single-line status strip at the bottom.
pub fn render_status_bar(frame: &mut Frame, area: Rect, state: &AppState) {
    let status_msg = state.status_message();
    let status_color = match &state.github.status {
        FetchStatus::Fetching => theme::WARNING,
        FetchStatus::Error(_) => theme::ERROR,
        FetchStatus::Success => theme::SUCCESS,
        FetchStatus::Idle => theme::SECONDARY,
    };

    let anim_indicator = if state.fx.animations_paused {
        Span::styled(" PAUSED ", Style::default().fg(theme::WARNING))
    } else if state.fx.should_animate() {
        let spinner_frames = ['·', '•', '●', '•'];
        let frame_idx = (state.fx.frame_count / 3) as usize % spinner_frames.len();
        Span::styled(format!(" {} ", spinner_frames[frame_idx]), theme::accent())
    } else {
        Span::raw(" ")
    };

    let rate_limit = &state.github.rate_limit;
    let rate_color = if rate_limit.is_low() {
        theme::ERROR
    } else {
        theme::SECONDARY
    };

    let divider = || Span::styled("│", Style::default().fg(theme::DIVIDER));
    let line = Line::from(vec![
        anim_indicator,
        divider(),
        Span::styled(
            format!(" {} ", status_msg),
            Style::default().fg(status_color),
        ),
        divider(),
        Span::styled(
            format!(" API {}/{} ", rate_limit.remaining, rate_limit.limit),
            Style::default().fg(rate_color),
        ),
        divider(),
        Span::styled(
            format!(
                " {}/5 {} ",
                state.ui.current_page.index() + 1,
                state.ui.current_page.title()
            ),
            theme::accent(),
        ),
        divider(),
        Span::styled(" 1–5 Navigate  ·  ? Help ", theme::secondary()),
    ]);

    frame.render_widget(Paragraph::new(line).style(theme::screen()), area);
}
