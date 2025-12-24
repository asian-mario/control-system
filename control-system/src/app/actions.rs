use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Actions that can be performed in the application
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Quit the application
    Quit,
    /// Force refresh GitHub data
    RefreshGithub,
    /// Go to next page
    NextPage,
    /// Go to previous page
    PrevPage,
    /// Go to a specific page (1-4)
    GoToPage(usize),
    /// Cycle focus between UI areas
    CycleFocus,
    /// Scroll up
    ScrollUp,
    /// Scroll down
    ScrollDown,
    /// Select next item in list
    SelectNext,
    /// Select previous item in list
    SelectPrev,
    /// Toggle help overlay
    ToggleHelp,
    /// Toggle animation pause
    TogglePause,
    /// No action
    None,
}

impl Action {
    /// Convert a key event to an action
    pub fn from_key_event(key: KeyEvent) -> Self {
        match key.code {
            // Quit
            KeyCode::Char('q') => Action::Quit,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
            
            // Refresh
            KeyCode::Char('r') => Action::RefreshGithub,
            
            // Page navigation
            KeyCode::Tab => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    Action::PrevPage
                } else {
                    Action::CycleFocus
                }
            }
            KeyCode::Char('1') => Action::GoToPage(0),
            KeyCode::Char('2') => Action::GoToPage(1),
            KeyCode::Char('3') => Action::GoToPage(2),
            KeyCode::Char('4') => Action::GoToPage(3),
            
            // Help
            KeyCode::Char('?') => Action::ToggleHelp,
            KeyCode::Char('h') => Action::ToggleHelp,
            
            // Pause animations
            KeyCode::Char('p') => Action::TogglePause,
            
            // Scrolling
            KeyCode::Up | KeyCode::Char('k') => Action::ScrollUp,
            KeyCode::Down | KeyCode::Char('j') => Action::ScrollDown,
            KeyCode::Left => Action::PrevPage,
            KeyCode::Right => Action::NextPage,
            
            // Selection
            KeyCode::Enter => Action::SelectNext,
            
            // Page up/down for faster scrolling
            KeyCode::PageUp => Action::ScrollUp,
            KeyCode::PageDown => Action::ScrollDown,
            
            _ => Action::None,
        }
    }
}

/// Get keybind help text
pub fn keybind_help() -> Vec<(&'static str, &'static str)> {
    vec![
        ("q", "Quit"),
        ("r", "Refresh GitHub"),
        ("1-4", "Switch pages"),
        ("Tab", "Cycle focus"),
        ("?/h", "Toggle help"),
        ("p", "Pause animations"),
        ("Up/k", "Scroll up"),
        ("Dn/j", "Scroll down"),
        ("L/R", "Prev/Next page"),
    ]
}
