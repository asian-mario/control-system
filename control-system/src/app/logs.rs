use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tracing_subscriber::fmt::MakeWriter;

/// Maximum number of log messages to keep
const MAX_LOG_MESSAGES: usize = 100;

/// A log message with level and content
#[derive(Debug, Clone)]
pub struct LogMessage {
    pub level: String,
    pub message: String,
}

/// Shared log buffer for the TUI
#[derive(Debug, Clone, Default)]
pub struct LogBuffer {
    messages: Arc<Mutex<VecDeque<LogMessage>>>,
}

impl LogBuffer {
    pub fn new() -> Self {
        Self {
            messages: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_LOG_MESSAGES))),
        }
    }

    pub fn push(&self, level: &str, message: String) {
        if let Ok(mut msgs) = self.messages.lock() {
            if msgs.len() >= MAX_LOG_MESSAGES {
                msgs.pop_front();
            }
            msgs.push_back(LogMessage {
                level: level.to_string(),
                message,
            });
        }
    }

    pub fn get_messages(&self) -> Vec<LogMessage> {
        if let Ok(msgs) = self.messages.lock() {
            msgs.iter().cloned().collect()
        } else {
            Vec::new()
        }
    }

    pub fn clear(&self) {
        if let Ok(mut msgs) = self.messages.lock() {
            msgs.clear();
        }
    }
}

/// Writer that captures logs to the buffer
pub struct LogWriter {
    buffer: LogBuffer,
    level: String,
}

impl LogWriter {
    pub fn new(buffer: LogBuffer, level: &str) -> Self {
        Self {
            buffer,
            level: level.to_string(),
        }
    }
}

impl std::io::Write for LogWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if let Ok(s) = std::str::from_utf8(buf) {
            let trimmed = s.trim();
            if !trimmed.is_empty() {
                self.buffer.push(&self.level, trimmed.to_string());
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Factory for creating log writers
#[derive(Clone)]
pub struct LogWriterFactory {
    buffer: LogBuffer,
}

impl LogWriterFactory {
    pub fn new(buffer: LogBuffer) -> Self {
        Self { buffer }
    }
}

impl<'a> MakeWriter<'a> for LogWriterFactory {
    type Writer = LogWriter;

    fn make_writer(&'a self) -> Self::Writer {
        LogWriter::new(self.buffer.clone(), "INFO")
    }
}
