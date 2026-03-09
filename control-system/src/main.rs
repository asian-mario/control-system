//! control-system - A TUI desk control dashboard for Raspberry Pi touchscreen
//!
//! This application provides a dynamic, animated dashboard that monitors GitHub stats
//! and system information, designed for use on a Raspberry Pi with a touchscreen.

mod app;
mod config;
mod github;
mod news;
mod spotify;
mod system;
mod ui;
mod util;

use std::io;
use std::io::stdout;
use std::panic;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, MouseButton,
        MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Terminal,
};
use tachyonfx::Effect;
use tokio::sync::mpsc;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

use app::{Action, AppState, LogBuffer, LogWriterFactory, Page};
use config::load::AppSettings;
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

    // Try loading config; if GITHUB_USER is missing, prompt for it
    let config = match Config::from_env_optional()? {
        Some(config) => config,
        None => {
            // Need to prompt user for GitHub username
            let mut terminal = setup_terminal()?;
            let username = prompt_github_user(&mut terminal)?;
            restore_terminal()?;
            if username.is_empty() {
                anyhow::bail!("GitHub username is required");
            }
            // Set for this process so pollers etc can find it
            std::env::set_var("GITHUB_USER", &username);
            // Save persistently so user doesn't need to re-enter
            let _ = AppSettings {
                github_user: username.clone(),
            }
            .save();
            Config::build_with_user(username)?
        }
    };

    info!("Starting control-system for user: {}", config.github_user);
    info!(
        "Refresh interval: {}s, Reduced motion: {}",
        config.refresh_secs, config.reduced_motion
    );

    // Check if Spotify is configured; if not, offer to set it up.
    // Also allow re-setup via SPOTIFY_RESET=1 env var.
    let spotify_reset = std::env::var("SPOTIFY_RESET").unwrap_or_default() == "1";
    if spotify_reset {
        info!("SPOTIFY_RESET=1 detected, clearing Spotify tokens");
        let _ = std::fs::remove_file(spotify::SpotifyAuth::token_path());
    }
    if !spotify::SpotifyAuth::is_configured() {
        let mut terminal = setup_terminal()?;
        let setup_result = prompt_spotify_setup(&mut terminal).await;
        restore_terminal()?;
        match setup_result {
            Ok(true) => info!("Spotify configured successfully"),
            Ok(false) => info!("Spotify setup skipped"),
            Err(e) => {
                // Remove any partial tokens
                let _ = std::fs::remove_file(spotify::SpotifyAuth::token_path());
                info!("Spotify setup failed: {}, continuing without Spotify", e);
            }
        }
    }

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

/// Try to open a URL in the user's default browser.
/// Returns true if the command was launched successfully.
fn open_url_in_browser(url: &str) -> bool {
    // Write a temp HTML file that auto-redirects to the URL.
    // This avoids shell interpretation of '&' in the URL which breaks
    // cmd.exe (Windows), WSL browser delegation, and some xdg-open configs.
    let html = format!(
        r#"<!DOCTYPE html><html><head><meta http-equiv="refresh" content="0;url={}"><script>window.location.href=decodeURIComponent("{}");</script></head><body>Redirecting to Spotify...</body></html>"#,
        html_escape(url),
        urlencoding::encode(url),
    );

    let redirect_path = std::env::temp_dir().join("control-system-spotify-redirect.html");
    if std::fs::write(&redirect_path, html).is_err() {
        return false;
    }

    let path_str = redirect_path.to_string_lossy().to_string();

    #[cfg(target_os = "linux")]
    let result = std::process::Command::new("xdg-open")
        .arg(&path_str)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();

    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open")
        .arg(&path_str)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();

    #[cfg(target_os = "windows")]
    let result = std::process::Command::new("cmd")
        .args(["/C", "start", "", &path_str])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    let result: std::io::Result<std::process::Child> = Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "unsupported OS",
    ));

    result.is_ok()
}

/// Escape special HTML characters in a string
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Prompt the user for their GitHub username with a themed TUI
fn prompt_github_user(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<String> {
    let mut input = String::new();
    let mut cursor_visible = true;
    let mut last_blink = Instant::now();

    loop {
        // Blink cursor every 500ms
        if last_blink.elapsed() >= Duration::from_millis(500) {
            cursor_visible = !cursor_visible;
            last_blink = Instant::now();
        }

        terminal.draw(|frame| {
            let size = frame.area();
            // Dark background
            frame.render_widget(Clear, size);
            let bg = Block::default().style(Style::default().bg(Color::Black));
            frame.render_widget(bg, size);

            // Center the prompt box
            let vert = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(30),
                    Constraint::Length(12),
                    Constraint::Percentage(30),
                ])
                .split(size);
            let horiz = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(20),
                    Constraint::Min(40),
                    Constraint::Percentage(20),
                ])
                .split(vert[1]);
            let box_area = horiz[1];

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(Span::styled(
                    " control-system ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ));

            let cursor_char = if cursor_visible { "_" } else { " " };

            let lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  GITHUB_USER not set.",
                    Style::default().fg(Color::Yellow),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  Enter your GitHub username:",
                    Style::default().fg(Color::White),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::styled("  > ", Style::default().fg(Color::Cyan)),
                    Span::styled(
                        input.as_str(),
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(cursor_char, Style::default().fg(Color::Cyan)),
                ]),
                Line::from(""),
                Line::from(Span::styled(
                    "  [Enter] confirm  [Esc] quit",
                    Style::default().fg(Color::DarkGray),
                )),
            ];

            let paragraph = Paragraph::new(lines).block(block);
            frame.render_widget(paragraph, box_area);
        })?;

        // Poll with short timeout for cursor blink
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Enter => {
                        let trimmed = input.trim().to_string();
                        return Ok(trimmed);
                    }
                    KeyCode::Esc => return Ok(String::new()),
                    KeyCode::Backspace => {
                        input.pop();
                    }
                    KeyCode::Char(c) if c.is_alphanumeric() || c == '-' || c == '_' => {
                        input.push(c);
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Prompt user to set up Spotify integration
async fn prompt_spotify_setup(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<bool> {
    // Phase 1: Ask if they want to set up Spotify
    let redirect_uri = spotify::SpotifyAuth::redirect_uri();
    let wants_setup = prompt_yes_no(
        terminal,
        &[
            ("", Color::Black),
            ("  Spotify is not configured.", Color::Yellow),
            ("", Color::Black),
            ("  To enable Spotify integration you need a", Color::White),
            ("  Spotify Developer App (Client ID).", Color::White),
            ("", Color::Black),
            (
                "  1. Go to: https://developer.spotify.com/dashboard",
                Color::Cyan,
            ),
            ("  2. Create an app (or select existing)", Color::White),
            ("  3. Add this Redirect URI in app settings:", Color::White),
            ("     http://127.0.0.1:8585/callback", Color::Green),
            ("", Color::Black),
            ("  Set up Spotify now? [y/n]", Color::White),
        ],
    )?;

    if !wants_setup {
        return Ok(false);
    }

    // Phase 2: Get Client ID
    let client_id = prompt_text_input(
        terminal,
        &[
            ("", Color::Black),
            ("  Enter your Spotify Client ID:", Color::White),
        ],
        "client_id",
    )?;

    if client_id.is_empty() {
        return Ok(false);
    }

    // Phase 3: Show auth URL, ask user to paste redirect URL back
    let (auth_url, verifier) = spotify::SpotifyAuth::build_auth_url(&client_id);

    // Try to open the URL in the user's browser automatically
    let browser_opened = open_url_in_browser(&auth_url);

    // Show a portion of the URL for reference (full URL is too long for TUI)
    let url_hint = if auth_url.len() > 60 {
        format!("{}...", &auth_url[..60])
    } else {
        auth_url.clone()
    };

    let browser_msg = if browser_opened {
        ("  [URL opened in your browser]", Color::Green)
    } else {
        ("  [Could not open browser -- see URL below]", Color::Yellow)
    };

    // If browser didn't open, write URL to a temp file so user can access it
    if !browser_opened {
        let url_path = std::env::temp_dir().join("control-system-spotify-auth-url.txt");
        let _ = std::fs::write(&url_path, &auth_url);
    }

    // Show URL screen with instructions
    let proceed = prompt_yes_no(
        terminal,
        &[
            ("", Color::Black),
            ("  Authorize Spotify in your browser:", Color::White),
            ("", Color::Black),
            browser_msg,
            ("", Color::Black),
            (&format!("  {}", url_hint), Color::DarkGray),
            ("", Color::Black),
            (
                "  If browser didn't open, the full URL was saved to:",
                Color::White,
            ),
            ("  /tmp/control-system-spotify-auth-url.txt", Color::Cyan),
            ("", Color::Black),
            (
                "  After authorizing, your browser will try to",
                Color::White,
            ),
            (
                "  load a page that won't connect -- that's OK!",
                Color::White,
            ),
            (
                "  Copy the FULL URL from your browser's address bar.",
                Color::Yellow,
            ),
            ("", Color::Black),
            ("  Ready to paste the URL? [y/n]", Color::White),
        ],
    )?;

    if !proceed {
        return Ok(false);
    }

    // Phase 4: Get the redirect URL with the code
    let redirect_url = prompt_paste_input(
        terminal,
        &[
            ("", Color::Black),
            (
                "  Paste the URL from your browser address bar:",
                Color::White,
            ),
            (
                "  (It should start with http://127.0.0.1:8585/callback?code=...)",
                Color::DarkGray,
            ),
        ],
    )?;

    if redirect_url.is_empty() {
        return Ok(false);
    }

    // Extract the authorization code from the URL
    let code = spotify::SpotifyAuth::extract_code_from_url(&redirect_url)?;

    // Exchange code for tokens
    terminal.draw(|frame| {
        let size = frame.area();
        frame.render_widget(Clear, size);
        let bg = Block::default().style(Style::default().bg(Color::Black));
        frame.render_widget(bg, size);

        let msg = Paragraph::new(Span::styled(
            "  Exchanging authorization code for tokens...",
            Style::default().fg(Color::Yellow),
        ));
        frame.render_widget(msg, Rect::new(2, size.height / 2, size.width - 4, 1));
    })?;

    spotify::SpotifyAuth::exchange_code(&client_id, &code, &verifier).await?;

    Ok(true)
}

/// Helper: yes/no prompt
fn prompt_yes_no(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    message_lines: &[(&str, Color)],
) -> Result<bool> {
    loop {
        let lines_clone: Vec<(String, Color)> = message_lines
            .iter()
            .map(|(s, c)| (s.to_string(), *c))
            .collect();
        terminal.draw(|frame| {
            let size = frame.area();
            frame.render_widget(Clear, size);
            let bg = Block::default().style(Style::default().bg(Color::Black));
            frame.render_widget(bg, size);

            let vert = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(25),
                    Constraint::Length(lines_clone.len() as u16 + 2),
                    Constraint::Percentage(25),
                ])
                .split(size);
            let horiz = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(15),
                    Constraint::Min(50),
                    Constraint::Percentage(15),
                ])
                .split(vert[1]);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green))
                .title(Span::styled(
                    " Spotify Setup ",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ));

            let lines: Vec<Line> = lines_clone
                .iter()
                .map(|(text, color)| {
                    Line::from(Span::styled(text.as_str(), Style::default().fg(*color)))
                })
                .collect();

            let paragraph = Paragraph::new(lines).block(block);
            frame.render_widget(paragraph, horiz[1]);
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => return Ok(true),
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => return Ok(false),
                    _ => {}
                }
            }
        }
    }
}

/// Helper: text input prompt
fn prompt_text_input(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    message_lines: &[(&str, Color)],
    _field_name: &str,
) -> Result<String> {
    let mut input = String::new();
    let mut cursor_visible = true;
    let mut last_blink = Instant::now();

    loop {
        if last_blink.elapsed() >= Duration::from_millis(500) {
            cursor_visible = !cursor_visible;
            last_blink = Instant::now();
        }

        let lines_clone: Vec<(String, Color)> = message_lines
            .iter()
            .map(|(s, c)| (s.to_string(), *c))
            .collect();
        let input_clone = input.clone();
        let cursor = if cursor_visible { "_" } else { " " };

        terminal.draw(|frame| {
            let size = frame.area();
            frame.render_widget(Clear, size);
            let bg = Block::default().style(Style::default().bg(Color::Black));
            frame.render_widget(bg, size);

            let vert = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(30),
                    Constraint::Length(lines_clone.len() as u16 + 5),
                    Constraint::Percentage(30),
                ])
                .split(size);
            let horiz = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(15),
                    Constraint::Min(50),
                    Constraint::Percentage(15),
                ])
                .split(vert[1]);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green))
                .title(Span::styled(
                    " Spotify Setup ",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ));

            let mut lines: Vec<Line> = lines_clone
                .iter()
                .map(|(text, color)| {
                    Line::from(Span::styled(text.as_str(), Style::default().fg(*color)))
                })
                .collect();

            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("  > ", Style::default().fg(Color::Green)),
                Span::styled(
                    input_clone.as_str(),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(cursor, Style::default().fg(Color::Green)),
            ]));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  [Enter] confirm  [Esc] cancel",
                Style::default().fg(Color::DarkGray),
            )));

            let paragraph = Paragraph::new(lines).block(block);
            frame.render_widget(paragraph, horiz[1]);
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Enter => return Ok(input.trim().to_string()),
                    KeyCode::Esc => return Ok(String::new()),
                    KeyCode::Backspace => {
                        input.pop();
                    }
                    KeyCode::Char(c) if c.is_alphanumeric() || c == '-' || c == '_' => {
                        input.push(c);
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Helper: text input that accepts any printable character (for URLs)
fn prompt_paste_input(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    message_lines: &[(&str, Color)],
) -> Result<String> {
    let mut input = String::new();
    let mut cursor_visible = true;
    let mut last_blink = Instant::now();

    loop {
        if last_blink.elapsed() >= Duration::from_millis(500) {
            cursor_visible = !cursor_visible;
            last_blink = Instant::now();
        }

        let lines_clone: Vec<(String, Color)> = message_lines
            .iter()
            .map(|(s, c)| (s.to_string(), *c))
            .collect();
        // Show truncated input if too long
        let max_display = 60;
        let display_input = if input.len() > max_display {
            format!("...{}", &input[input.len() - max_display..])
        } else {
            input.clone()
        };
        let cursor = if cursor_visible { "_" } else { " " };

        terminal.draw(|frame| {
            let size = frame.area();
            frame.render_widget(Clear, size);
            let bg = Block::default().style(Style::default().bg(Color::Black));
            frame.render_widget(bg, size);

            let vert = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(25),
                    Constraint::Length(lines_clone.len() as u16 + 6),
                    Constraint::Percentage(25),
                ])
                .split(size);
            let horiz = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(10),
                    Constraint::Min(50),
                    Constraint::Percentage(10),
                ])
                .split(vert[1]);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green))
                .title(Span::styled(
                    " Spotify Setup ",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ));

            let mut lines: Vec<Line> = lines_clone
                .iter()
                .map(|(text, color)| {
                    Line::from(Span::styled(text.as_str(), Style::default().fg(*color)))
                })
                .collect();

            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("  > ", Style::default().fg(Color::Green)),
                Span::styled(
                    display_input.as_str(),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(cursor, Style::default().fg(Color::Green)),
            ]));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  [Enter] confirm  [Esc] cancel",
                Style::default().fg(Color::DarkGray),
            )));

            let paragraph = Paragraph::new(lines).block(block);
            frame.render_widget(paragraph, horiz[1]);
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Enter => return Ok(input.trim().to_string()),
                    KeyCode::Esc => return Ok(String::new()),
                    KeyCode::Backspace => {
                        input.pop();
                    }
                    KeyCode::Char(c) => {
                        input.push(c);
                    }
                    _ => {}
                }
            }
        }
    }
}

/// MS-DOS style bootup sequence with typing animation
fn run_bootup_sequence(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    username: &str,
) -> Result<()> {
    let user_line = format!("C:\\USERS\\{}> cd RO:CS", username.to_uppercase());
    let boot_lines: Vec<(&str, Color, u64)> = vec![
        ("RO:CS v0.1.0", Color::White, 30),
        ("(C) 2026 RO:CS", Color::DarkGray, 20),
        ("", Color::Black, 0),
        (
            "BIOS Date 12/25/24 08:00:00 Ver: 08.00.00",
            Color::DarkGray,
            15,
        ),
        ("Checking RAM.......... 640K OK", Color::White, 10),
        ("", Color::Black, 0),
        (&user_line, Color::Gray, 25),
        ("C:\\CONTROL-SYSTEM> init --modules=all", Color::Gray, 25),
        ("", Color::Black, 0),
        ("Loading GitHub module........... [OK]", Color::Green, 15),
        ("Loading system monitor.......... [OK]", Color::Green, 15),
        ("Loading news feed............... [OK]", Color::Green, 15),
        ("Loading TUI renderer............ [OK]", Color::Green, 15),
        ("Fetching APIs........ [OK]", Color::Green, 15),
        ("", Color::Black, 0),
        ("STARTING....", Color::Cyan, 20),
    ];

    // Collect the lines we'll display with typing effect
    let mut displayed_lines: Vec<(String, Color)> = Vec::new();

    for (text, color, char_delay_ms) in &boot_lines {
        if text.is_empty() {
            displayed_lines.push((String::new(), *color));
            render_boot_screen(terminal, &displayed_lines)?;
            std::thread::sleep(Duration::from_millis(80));
            continue;
        }

        // Type out each character
        let mut current = String::new();
        displayed_lines.push((String::new(), *color));
        let line_idx = displayed_lines.len() - 1;

        for ch in text.chars() {
            current.push(ch);
            displayed_lines[line_idx].0 = current.clone();
            render_boot_screen(terminal, &displayed_lines)?;
            std::thread::sleep(Duration::from_millis(*char_delay_ms));

            // Allow Esc to skip
            if event::poll(Duration::from_millis(0))? {
                if let Event::Key(key) = event::read()? {
                    if key.code == KeyCode::Esc || key.code == KeyCode::Enter {
                        // Fast-forward: fill all remaining
                        displayed_lines[line_idx].0 = text.to_string();
                        render_boot_screen(terminal, &displayed_lines)?;
                        // Skip the rest of typing but still show all lines
                        return finish_boot_fast(
                            terminal,
                            &mut displayed_lines,
                            &boot_lines,
                            line_idx + 1,
                        );
                    }
                }
            }
        }

        // Brief pause between lines
        std::thread::sleep(Duration::from_millis(100));
    }

    // Hold the final screen briefly, then fade transition
    std::thread::sleep(Duration::from_millis(600));

    // Fade out effect - render progressively darker
    for step in 0..8 {
        terminal.draw(|frame| {
            let size = frame.area();
            let block = Block::default().style(Style::default().bg(Color::Black));
            frame.render_widget(block, size);

            if step < 6 {
                let lines: Vec<Line> = displayed_lines
                    .iter()
                    .map(|(text, _)| {
                        Line::from(Span::styled(
                            text.as_str(),
                            Style::default().fg(Color::DarkGray),
                        ))
                    })
                    .collect();
                let paragraph = Paragraph::new(lines).style(Style::default().bg(Color::Black));
                frame.render_widget(
                    paragraph,
                    Rect::new(
                        2,
                        1,
                        size.width.saturating_sub(4),
                        size.height.saturating_sub(2),
                    ),
                );
            }
        })?;
        std::thread::sleep(Duration::from_millis(60));
    }

    Ok(())
}

/// Fast-forward remaining boot lines (when user skips)
fn finish_boot_fast(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    displayed_lines: &mut Vec<(String, Color)>,
    boot_lines: &[(&str, Color, u64)],
    start_from: usize,
) -> Result<()> {
    for (text, color, _) in boot_lines.iter().skip(start_from) {
        displayed_lines.push((text.to_string(), *color));
    }
    render_boot_screen(terminal, displayed_lines)?;
    std::thread::sleep(Duration::from_millis(400));
    Ok(())
}

/// Render the boot screen with accumulated lines
fn render_boot_screen(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    lines: &[(String, Color)],
) -> Result<()> {
    let lines_clone: Vec<(String, Color)> = lines.to_vec();
    terminal.draw(|frame| {
        let size = frame.area();
        let bg = Block::default().style(Style::default().bg(Color::Black));
        frame.render_widget(bg, size);

        let content_area = Rect::new(
            2,
            1,
            size.width.saturating_sub(4),
            size.height.saturating_sub(2),
        );

        let display: Vec<Line> = lines_clone
            .iter()
            .map(|(text, color)| {
                if text.is_empty() {
                    Line::from("")
                } else {
                    Line::from(Span::styled(text.as_str(), Style::default().fg(*color)))
                }
            })
            .collect();

        let paragraph = Paragraph::new(display).style(Style::default().bg(Color::Black));
        frame.render_widget(paragraph, content_area);
    })?;
    Ok(())
}

/// Main application loop
async fn run_app(config: Config, log_buffer: LogBuffer) -> Result<()> {
    // Query terminal image protocol capabilities before entering alternate screen
    let mut picker = ratatui_image::picker::Picker::from_query_stdio().ok();

    // Set up terminal
    let mut terminal = setup_terminal()?;

    // Run MS-DOS style bootup sequence
    run_bootup_sequence(&mut terminal, &config.github_user)?;

    // Initialize app state
    let mut state = AppState::new(config.reduced_motion, log_buffer);

    // Set up GitHub poller
    let poller = GithubPoller::new(&config)?;
    let initial_github_state = poller.load_cached_state().await;
    state.github = initial_github_state.clone();

    let (github_rx, github_cmd_tx) = poller.start(initial_github_state);

    // Set up system stats poller
    let system_rx = SystemStats::start_poller(Duration::from_secs(2));

    // Set up news feed poller (refresh every 5 minutes)
    let news_rx = news::NewsPoller::start(Duration::from_secs(300));

    // Set up Spotify poller (poll every second for smooth progress bar)
    let (spotify_rx, spotify_cmd_tx) = if spotify::SpotifyAuth::is_configured() {
        let (rx, tx) = spotify::SpotifyPoller::start(Duration::from_secs(1));
        (Some(rx), Some(tx))
    } else {
        info!("Spotify not configured, skipping poller");
        (None, None)
    };

    // Set up internal command channel
    let (action_tx, mut action_rx) = mpsc::channel::<Action>(32);

    // Active effects
    let mut effects: Vec<Effect> = Vec::new();

    // Album art image protocol state (for ratatui-image)
    let mut album_art_proto: Option<ratatui_image::protocol::StatefulProtocol> = None;
    let mut cached_art_url: Option<String> = None;

    // Frame timing
    let mut last_frame = Instant::now();

    info!("control-system started, entering main loop");

    // Main event loop
    while state.running {
        let frame_start = Instant::now();

        // Read terminal events on the main thread (single-threaded to avoid
        // Windows console deadlock and WSL terminal corruption)
        while event::poll(Duration::ZERO)? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Press {
                        let action = Action::from_key_event(key);
                        let _ = action_tx.try_send(action);
                    }
                }
                Event::Mouse(mouse) => {
                    if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
                        if mouse.row == 1 {
                            let col = mouse.column;
                            let clicked_tab = if col >= 1 && col <= 14 {
                                Some(0)
                            } else if col >= 15 && col <= 24 {
                                Some(1)
                            } else if col >= 25 && col <= 37 {
                                Some(2)
                            } else if col >= 38 && col <= 49 {
                                Some(3)
                            } else if col >= 50 {
                                Some(4)
                            } else {
                                None
                            };
                            if let Some(tab) = clicked_tab {
                                let _ = action_tx.try_send(Action::GoToPage(tab));
                            }
                        }

                        if let Some(spotify_action) =
                            ui::widgets::spotify_player::check_spotify_click(
                                mouse.column,
                                mouse.row,
                            )
                        {
                            let action = match spotify_action {
                                ui::widgets::spotify_player::SpotifyAction::Toggle => {
                                    Action::SpotifyToggle
                                }
                                ui::widgets::spotify_player::SpotifyAction::Prev => {
                                    Action::SpotifyPrev
                                }
                                ui::widgets::spotify_player::SpotifyAction::Next => {
                                    Action::SpotifyNext
                                }
                            };
                            let _ = action_tx.try_send(action);
                        }
                    }
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
                    let _ = github_cmd_tx.try_send(github::GithubCommand::Refresh);
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
                Action::SpotifyToggle => {
                    if let Some(ref tx) = spotify_cmd_tx {
                        let _ = tx.try_send(spotify::SpotifyCommand::TogglePlayback);
                    }
                }
                Action::SpotifyNext => {
                    if let Some(ref tx) = spotify_cmd_tx {
                        let _ = tx.try_send(spotify::SpotifyCommand::NextTrack);
                    }
                }
                Action::SpotifyPrev => {
                    if let Some(ref tx) = spotify_cmd_tx {
                        let _ = tx.try_send(spotify::SpotifyCommand::PrevTrack);
                    }
                }
                Action::SpotifyReset => {
                    if state.ui.current_page == Page::Settings {
                        info!("Spotify reset requested from settings");
                        let _ = std::fs::remove_file(spotify::SpotifyAuth::token_path());
                        // Stop the poller
                        if let Some(ref tx) = spotify_cmd_tx {
                            let _ = tx.try_send(spotify::SpotifyCommand::Stop);
                        }
                        state.spotify = spotify::SpotifyState::default();
                        info!("Spotify tokens cleared. Restart app to re-configure.");
                    }
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

        if news_rx.has_changed().unwrap_or(false) {
            state.news = news_rx.borrow().clone();
        }

        // Update Spotify state
        if let Some(ref rx) = spotify_rx {
            if rx.has_changed().unwrap_or(false) {
                state.spotify = rx.borrow().clone();
            }
        }

        // Update album art protocol if art URL changed
        let current_art_url = state.spotify.player.album_art_url.clone();
        if current_art_url != cached_art_url {
            album_art_proto = if let (Some(ref mut p), Some(art)) =
                (&mut picker, &state.spotify.player.album_art)
            {
                image::RgbaImage::from_raw(art.width, art.height, art.rgba_data.clone())
                    .map(|rgba| p.new_resize_protocol(image::DynamicImage::ImageRgba8(rgba)))
            } else {
                None
            };
            cached_art_url = current_art_url;
        }

        let delta_ms = last_frame.elapsed().as_millis() as f32;

        // Update animation state
        state.fx.tick(delta_ms);
        last_frame = Instant::now();

        // Render
        terminal.draw(|frame| {
            render_app(frame, &state, &mut effects, &mut album_art_proto);
        })?;

        // Frame rate limiting
        let frame_time = frame_start.elapsed();
        if frame_time < FRAME_DURATION {
            tokio::time::sleep(FRAME_DURATION - frame_time).await;
        }
    }

    // Clean up
    let _ = github_cmd_tx.try_send(github::GithubCommand::Stop);
    if let Some(ref tx) = spotify_cmd_tx {
        let _ = tx.try_send(spotify::SpotifyCommand::Stop);
    }
    info!("control-system shutdown complete");

    Ok(())
}
