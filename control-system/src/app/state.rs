use crate::app::logs::LogBuffer;
use crate::github::GithubState;
use crate::system::stats::SystemState;

/// The current page being displayed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Page {
    #[default]
    Dashboard,
    Repositories,
    Activity,
    Settings,
}

impl Page {
    pub fn title(&self) -> &'static str {
        match self {
            Page::Dashboard => "Dashboard",
            Page::Repositories => "Repositories",
            Page::Activity => "Activity Feed",
            Page::Settings => "Settings & Help",
        }
    }

    pub fn index(&self) -> usize {
        match self {
            Page::Dashboard => 0,
            Page::Repositories => 1,
            Page::Activity => 2,
            Page::Settings => 3,
        }
    }

    pub fn from_index(index: usize) -> Self {
        match index {
            0 => Page::Dashboard,
            1 => Page::Repositories,
            2 => Page::Activity,
            3 => Page::Settings,
            _ => Page::Dashboard,
        }
    }

    pub fn next(&self) -> Self {
        Self::from_index((self.index() + 1) % 4)
    }

    pub fn prev(&self) -> Self {
        Self::from_index((self.index() + 3) % 4)
    }
}

/// UI-specific state
#[derive(Debug, Clone, Default)]
pub struct UiState {
    pub current_page: Page,
    pub show_help_overlay: bool,
    pub scroll_offset: usize,
    pub selected_index: usize,
    pub focus_area: FocusArea,
}

/// Which area of the UI has focus
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusArea {
    #[default]
    Main,
    Sidebar,
    List,
}

impl FocusArea {
    pub fn next(&self) -> Self {
        match self {
            FocusArea::Main => FocusArea::Sidebar,
            FocusArea::Sidebar => FocusArea::List,
            FocusArea::List => FocusArea::Main,
        }
    }
}

/// Animation/effects state
#[derive(Debug, Clone)]
pub struct FxState {
    pub animations_paused: bool,
    pub reduced_motion: bool,
    pub frame_count: u64,
    pub last_transition_frame: u64,
    pub transition_active: bool,
    pub pulse_phase: f32,
    pub shimmer_offset: f32,
}

impl Default for FxState {
    fn default() -> Self {
        Self {
            animations_paused: false,
            reduced_motion: false,
            frame_count: 0,
            last_transition_frame: 0,
            transition_active: false,
            pulse_phase: 0.0,
            shimmer_offset: 0.0,
        }
    }
}

impl FxState {
    /// Check if animations should play
    pub fn should_animate(&self) -> bool {
        !self.animations_paused && !self.reduced_motion
    }

    /// Update animation state for a new frame
    pub fn tick(&mut self, delta_ms: f32) {
        self.frame_count += 1;
        
        if self.should_animate() {
            // Pulse animation (breathing effect)
            self.pulse_phase = (self.pulse_phase + delta_ms * 0.003) % (2.0 * std::f32::consts::PI);
            
            // Shimmer animation
            self.shimmer_offset = (self.shimmer_offset + delta_ms * 0.05) % 100.0;
        }

        // Check if transition is complete
        if self.transition_active && self.frame_count - self.last_transition_frame > 20 {
            self.transition_active = false;
        }
    }

    /// Start a page transition
    pub fn start_transition(&mut self) {
        self.transition_active = true;
        self.last_transition_frame = self.frame_count;
    }

    /// Get current pulse value (0.0 to 1.0)
    pub fn pulse_value(&self) -> f32 {
        if self.should_animate() {
            (self.pulse_phase.sin() + 1.0) / 2.0
        } else {
            0.5
        }
    }
}

/// Complete application state
#[derive(Debug, Clone)]
pub struct AppState {
    pub github: GithubState,
    pub system: SystemState,
    pub ui: UiState,
    pub fx: FxState,
    pub log_buffer: LogBuffer,
    pub running: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            github: GithubState::default(),
            system: SystemState::default(),
            ui: UiState::default(),
            fx: FxState::default(),
            log_buffer: LogBuffer::new(),
            running: true,
        }
    }
}

impl AppState {
    /// Create new app state with config
    pub fn new(reduced_motion: bool, log_buffer: LogBuffer) -> Self {
        let mut state = Self::default();
        state.fx.reduced_motion = reduced_motion;
        state.log_buffer = log_buffer;
        state
    }

    /// Check if we have any GitHub data loaded
    pub fn has_github_data(&self) -> bool {
        self.github.profile.is_some() || !self.github.repos.is_empty()
    }

    /// Get status message for the status bar
    pub fn status_message(&self) -> String {
        use crate::github::FetchStatus;
        
        match &self.github.status {
            FetchStatus::Idle => {
                if let Some(last_updated) = self.github.last_updated {
                    format!("Last updated: {}", crate::util::time::format_relative(last_updated))
                } else {
                    "No data loaded".to_string()
                }
            }
            FetchStatus::Fetching => "Fetching GitHub data...".to_string(),
            FetchStatus::Success => {
                if let Some(last_updated) = self.github.last_updated {
                    format!("Updated: {}", crate::util::time::format_relative(last_updated))
                } else {
                    "Data loaded".to_string()
                }
            }
            FetchStatus::Error(e) => format!("Error: {}", e),
        }
    }
}
