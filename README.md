# DeskPilot TUI

A **TUI-only** desk control dashboard designed for **Raspberry Pi touchscreen**. Dynamic, animated, and constantly monitoring your GitHub stats.

![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)
![Platform](https://img.shields.io/badge/platform-x86__64%20%7C%20ARM-blue.svg)

## Features

- 📊 **GitHub Dashboard** - Real-time stats: stars, forks, repos, followers
- 📦 **Repository Spotlight** - Top starred and recently updated repos
- 📡 **Activity Feed** - GitHub events with new activity highlighting
- 💻 **System Stats** - CPU, memory, uptime monitoring
- 🕐 **Clock Widget** - Time and date display
- 🎨 **Animated UI** - Smooth transitions, breathing pulses, visual effects
- ⚡ **Non-blocking** - Async design, never freezes on network I/O
- 💾 **Caching** - Loads instantly from cache, updates in background

## Quick Start

### Prerequisites

```bash
# Install Rust targets
rustup target add x86_64-unknown-linux-gnu
rustup target add aarch64-unknown-linux-gnu
rustup target add armv7-unknown-linux-gnueabihf
```

### Set Environment Variables

```bash
export GITHUB_USER="your-username"           # Required
export GITHUB_TOKEN="your-token"             # Recommended (higher rate limits)
export DESKPILOT_REFRESH_SECS=60            # Optional (default: 60)
export DESKPILOT_REDUCED_MOTION=false       # Optional (default: false)
```

### Build & Run

```bash
# Development build
cd control-system
cargo run

# Release build (optimized)
cargo build --release
./target/release/deskpilot
```

### Cross Compile for Raspberry Pi

```bash
# Install cross
cargo install cross --locked

# 64-bit Raspberry Pi OS
cross build --release --target aarch64-unknown-linux-gnu

# 32-bit Raspberry Pi OS
cross build --release --target armv7-unknown-linux-gnueabihf
```

## Keyboard Controls

| Key | Action |
|-----|--------|
| `q` | Quit |
| `r` | Refresh GitHub data |
| `1-4` | Switch pages |
| `Tab` | Cycle focus |
| `?` / `h` | Toggle help |
| `p` | Pause animations |
| `↑` / `k` | Scroll up |
| `↓` / `j` | Scroll down |
| `←` / `→` | Previous/Next page |

## Pages

1. **Dashboard** - Overview with GitHub stats, clock, system info
2. **Repositories** - Top starred and recently updated repos
3. **Activity** - GitHub events feed
4. **Settings** - Keybinds, animation toggle, rate limit info

## Configuration

| Environment Variable | Description | Default |
|---------------------|-------------|---------|
| `GITHUB_USER` | GitHub username (required) | - |
| `GITHUB_TOKEN` | GitHub personal access token | - |
| `DESKPILOT_REFRESH_SECS` | Auto-refresh interval (seconds) | 60 |
| `DESKPILOT_REDUCED_MOTION` | Disable animations | false |

## Cache Location

Cache is stored at:
- Primary: `~/.config/deskpilot/cache.json`
- Fallback: `./deskpilot-cache.json`

## Tech Stack

- **ratatui** - TUI framework
- **crossterm** - Terminal handling
- **tokio** - Async runtime
- **tachyonfx** - Animation effects
- **octocrab** - GitHub API
- **sysinfo** - System monitoring

## License

See [LICENSE.md](LICENSE.md)
