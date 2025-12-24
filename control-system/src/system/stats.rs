use sysinfo::{System, CpuRefreshKind, MemoryRefreshKind, RefreshKind};
use std::time::Duration;
use tokio::sync::watch;
use tracing::debug;

/// System statistics data
#[derive(Debug, Clone, Default)]
pub struct SystemState {
    pub cpu_usage: f32,
    pub memory_used: u64,
    pub memory_total: u64,
    pub memory_percent: f32,
    pub uptime_secs: u64,
    pub hostname: String,
    pub os_name: String,
    pub cpu_count: usize,
}

impl SystemState {
    /// Format uptime as human-readable string
    pub fn uptime_formatted(&self) -> String {
        let secs = self.uptime_secs;
        let days = secs / 86400;
        let hours = (secs % 86400) / 3600;
        let mins = (secs % 3600) / 60;

        if days > 0 {
            format!("{}d {}h {}m", days, hours, mins)
        } else if hours > 0 {
            format!("{}h {}m", hours, mins)
        } else {
            format!("{}m", mins)
        }
    }
}

/// System stats collector and poller
pub struct SystemStats {
    system: System,
}

impl SystemStats {
    /// Create a new system stats collector
    pub fn new() -> Self {
        let system = System::new_with_specifics(
            RefreshKind::new()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything()),
        );

        Self { system }
    }

    /// Collect current system stats
    pub fn collect(&mut self) -> SystemState {
        // Refresh the data we need
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();

        let cpu_usage = self.system.global_cpu_usage();
        let memory_used = self.system.used_memory();
        let memory_total = self.system.total_memory();
        let memory_percent = if memory_total > 0 {
            (memory_used as f32 / memory_total as f32) * 100.0
        } else {
            0.0
        };

        SystemState {
            cpu_usage,
            memory_used,
            memory_total,
            memory_percent,
            uptime_secs: System::uptime(),
            hostname: System::host_name().unwrap_or_else(|| "unknown".to_string()),
            os_name: System::name().unwrap_or_else(|| "unknown".to_string()),
            cpu_count: self.system.cpus().len(),
        }
    }

    /// Start a background poller for system stats
    pub fn start_poller(
        poll_interval: Duration,
    ) -> watch::Receiver<SystemState> {
        let (tx, rx) = watch::channel(SystemState::default());

        tokio::spawn(async move {
            let mut stats = SystemStats::new();
            let mut interval = tokio::time::interval(poll_interval);

            loop {
                interval.tick().await;
                let state = stats.collect();
                debug!(
                    "System stats: CPU {:.1}%, MEM {:.1}%",
                    state.cpu_usage, state.memory_percent
                );
                if tx.send(state).is_err() {
                    break;
                }
            }
        });

        rx
    }
}

impl Default for SystemStats {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uptime_formatted() {
        let state = SystemState {
            uptime_secs: 90061, // 1 day, 1 hour, 1 minute, 1 second
            ..Default::default()
        };
        assert_eq!(state.uptime_formatted(), "1d 1h 1m");

        let state2 = SystemState {
            uptime_secs: 3661, // 1 hour, 1 minute, 1 second
            ..Default::default()
        };
        assert_eq!(state2.uptime_formatted(), "1h 1m");

        let state3 = SystemState {
            uptime_secs: 125, // 2 minutes, 5 seconds
            ..Default::default()
        };
        assert_eq!(state3.uptime_formatted(), "2m");
    }

    #[test]
    fn test_collect_stats() {
        let mut stats = SystemStats::new();
        let state = stats.collect();
        
        // Basic sanity checks
        assert!(state.memory_total > 0);
        assert!(state.cpu_count > 0);
    }
}
