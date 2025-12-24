use crossterm::event::{Event as CrosstermEvent, KeyEvent, MouseEvent};

/// Application events
#[derive(Debug, Clone)]
pub enum AppEvent {
    /// A key was pressed
    Key(KeyEvent),
    /// Mouse event
    Mouse(MouseEvent),
    /// Terminal resize
    Resize(u16, u16),
    /// Render tick
    Tick,
    /// GitHub data was updated
    GithubUpdated,
    /// System stats were updated  
    SystemUpdated,
}

impl From<CrosstermEvent> for AppEvent {
    fn from(event: CrosstermEvent) -> Self {
        match event {
            CrosstermEvent::Key(key) => AppEvent::Key(key),
            CrosstermEvent::Mouse(mouse) => AppEvent::Mouse(mouse),
            CrosstermEvent::Resize(w, h) => AppEvent::Resize(w, h),
            _ => AppEvent::Tick,
        }
    }
}
