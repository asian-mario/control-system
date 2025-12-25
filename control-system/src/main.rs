//! control-system - A TUI desk control dashboard for Raspberry Pi touchscreen
//!
//! This application provides a dynamic, animated dashboard that monitors GitHub stats
//! and system information, designed for use on a Raspberry Pi with a touchscreen.

mod app;
mod config;
mod github;
mod system;
mod ui;
mod util;

use std::io;
use std::io::stdout;
use std::panic;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind, MouseEventKind, MouseButton},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tachyonfx::Effect;
use tokio::sync::mpsc;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

use app::{Action, AppState, Page, LogBuffer, LogWriterFactory};
use config::Config;
use github::GithubPoller;
use system::SystemStats;
use ui::render_app;

/// Target frame rate for the UI
const TARGET_FPS: u64 = 30;
const FRAME_DURATION: Duration = Duration::from_millis(1000 / TARGET_FPS);

#[tokio::main]
async fn main() -> Result<()> {
    // Set up log buffer for TUI display
    let log_buffer = LogBuffer::new();
    let log_writer = LogWriterFactory::new(log_buffer.clone());

    // Set up logging to the buffer instead of stderr
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_writer(log_writer)
        .with_ansi(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // Set up panic hook to restore terminal on panic
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = restore_terminal();
        original_hook(panic_info);
    }));

    // Load configuration
    let config = Config::from_env()?;
    info!("Starting control-system for user: {}", config.github_user);
    info!(
        "Refresh interval: {}s, Reduced motion: {}",
        config.refresh_secs, config.reduced_motion
    );

    // Run the application
    let result = run_app(config, log_buffer).await;

    // Restore terminal
    restore_terminal()?;

    if let Err(ref e) = result {
        error!("Application error: {}", e);
    }

    result
}

/// Set up the terminal for the TUI
fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

/// Restore the terminal to its original state
fn restore_terminal() -> Result<()> {
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}

/// Main application loop
async fn run_app(config: Config, log_buffer: LogBuffer) -> Result<()> {
    // Set up terminal
    let mut terminal = setup_terminal()?;

    // Initialize app state
    let mut state = AppState::new(config.reduced_motion, log_buffer);

    // Set up GitHub poller
    let poller = GithubPoller::new(&config)?;
    let initial_github_state = poller.load_cached_state().await;
    state.github = initial_github_state.clone();

    let (github_rx, github_cmd_tx) = poller.start(initial_github_state);

    // Set up system stats poller
    let system_rx = SystemStats::start_poller(Duration::from_secs(2));

    // Set up internal command channel
    let (action_tx, mut action_rx) = mpsc::channel::<Action>(32);

    // Active effects
    let mut effects: Vec<Effect> = Vec::new();

    // Frame timing
    let mut last_frame = Instant::now();

    info!("control-system started, entering main loop");

    // Main event loop
    while state.running {
        let frame_start = Instant::now();

        // Poll for terminal events (non-blocking)
        // Drain all pending events to prevent lag buildup
        while event::poll(Duration::from_millis(0))? {
            match event::read()? {
                Event::Key(key) => {
                    // Only handle key press events (not release)
                    if key.kind == KeyEventKind::Press {
                        let action = Action::from_key_event(key);
                        let _ = action_tx.try_send(action);
                    }
                }
                Event::Mouse(mouse) => {
                    // Only handle left mouse button clicks (ignore move, drag, scroll)
                    if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
                        // Check if click is in header area (row 1, inside the border)
                        if mouse.row == 1 {
                            // Tab layout after left border (col 1):
                            // "1:Dashboard | 2:Repos | 3:Activity | 4:Settings"
                            // Positions: 1-11, 15-21, 25-34, 38-47
                            let col = mouse.column;
                            let clicked_tab = if col >= 1 && col <= 14 {
                                Some(0) // 1:Dashboard
                            } else if col >= 15 && col <= 24 {
                                Some(1) // 2:Repos
                            } else if col >= 25 && col <= 37 {
                                Some(2) // 3:Activity
                            } else if col >= 38 {
                                Some(3) // 4:Settings
                            } else {
                                None
                            };
                            
                            if let Some(tab) = clicked_tab {
                                let _ = action_tx.try_send(Action::GoToPage(tab));
                            }
                        }
                    }
                    // Ignore all other mouse events (move, scroll, etc.) to prevent lag
                }
                Event::Resize(_width, _height) => {
                    // Terminal resized - nothing special needed
                }
                _ => {}
            }
        }

        // Process actions
        while let Ok(action) = action_rx.try_recv() {
            match action {
                Action::Quit => {
                    info!("Quit requested");
                    state.running = false;
                }
                Action::RefreshGithub => {
                    info!("Manual refresh requested");
                    let _ = github_cmd_tx
                        .send(github::GithubCommand::Refresh)
                        .await;
                }
                Action::NextPage => {
                    let old_page = state.ui.current_page;
                    state.ui.current_page = state.ui.current_page.next();
                    if old_page != state.ui.current_page {
                        state.fx.start_transition();
                        state.ui.scroll_offset = 0;
                    }
                }
                Action::PrevPage => {
                    let old_page = state.ui.current_page;
                    state.ui.current_page = state.ui.current_page.prev();
                    if old_page != state.ui.current_page {
                        state.fx.start_transition();
                        state.ui.scroll_offset = 0;
                    }
                }
                Action::GoToPage(index) => {
                    let new_page = Page::from_index(index);
                    if state.ui.current_page != new_page {
                        state.ui.current_page = new_page;
                        state.fx.start_transition();
                        state.ui.scroll_offset = 0;
                    }
                }
                Action::CycleFocus => {
                    state.ui.focus_area = state.ui.focus_area.next();
                }
                Action::ToggleHelp => {
                    state.ui.show_help_overlay = !state.ui.show_help_overlay;
                }
                Action::TogglePause => {
                    state.fx.animations_paused = !state.fx.animations_paused;
                    info!("Animations paused: {}", state.fx.animations_paused);
                }
                Action::ScrollUp => {
                    state.ui.scroll_offset = state.ui.scroll_offset.saturating_sub(1);
                }
                Action::ScrollDown => {
                    state.ui.scroll_offset = state.ui.scroll_offset.saturating_add(1);
                }
                Action::SelectNext => {
                    state.ui.selected_index = state.ui.selected_index.saturating_add(1);
                }
                Action::SelectPrev => {
                    state.ui.selected_index = state.ui.selected_index.saturating_sub(1);
                }
                Action::None => {}
            }
        }

        // Update state from pollers
        if github_rx.has_changed().unwrap_or(false) {
            let new_github = github_rx.borrow().clone();
            
            // Check for new events and trigger effects
            if state.fx.should_animate() {
                let new_event_count = new_github.events.iter().filter(|e| e.is_new).count();
                if new_event_count > 0 {
                    // Could add pulse effect here for new events
                }
            }
            
            state.github = new_github;
        }

        if system_rx.has_changed().unwrap_or(false) {
            state.system = system_rx.borrow().clone();
        }

        // Update animation state
        let delta_ms = last_frame.elapsed().as_millis() as f32;
        state.fx.tick(delta_ms);
        last_frame = Instant::now();

        // Render
        terminal.draw(|frame| {
            render_app(frame, &state, &mut effects);
        })?;

        // Frame rate limiting
        let frame_time = frame_start.elapsed();
        if frame_time < FRAME_DURATION {
            tokio::time::sleep(FRAME_DURATION - frame_time).await;
        }
    }

    // Clean up
    let _ = github_cmd_tx.send(github::GithubCommand::Stop).await;
    info!("control-system shutdown complete");

    Ok(())
}
