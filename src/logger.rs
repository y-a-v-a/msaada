// src/logger.rs
// Advanced logging system with colored output and timestamps

use chrono::{DateTime, Local};
use colored::*;
use std::fmt;

#[derive(Debug, Clone, Copy)]
pub enum LogLevel {
    Http,
    Info,
    Warn,
    Error,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LogLevel::Http => write!(f, "{}", " HTTP ".on_blue().bold().white()),
            LogLevel::Info => write!(f, "{}", " INFO ".on_magenta().bold().white()),
            LogLevel::Warn => write!(f, "{}", " WARN ".on_yellow().bold().black()),
            LogLevel::Error => write!(f, "{}", " ERROR ".on_red().bold().white()),
        }
    }
}

pub struct Logger {
    pub enable_request_logging: bool,
    pub enable_timestamps: bool,
}

impl Logger {
    pub fn new() -> Self {
        Self {
            enable_request_logging: true,
            enable_timestamps: true,
        }
    }

    pub fn with_request_logging(mut self, enable: bool) -> Self {
        self.enable_request_logging = enable;
        self
    }

    pub fn with_timestamps(mut self, enable: bool) -> Self {
        self.enable_timestamps = enable;
        self
    }

    fn format_timestamp(&self) -> String {
        if self.enable_timestamps {
            let now: DateTime<Local> = Local::now();
            format!("{} ", now.format("%Y-%m-%d %H:%M:%S").to_string().dimmed())
        } else {
            String::new()
        }
    }

    pub fn log(&self, level: LogLevel, message: &str) {
        let timestamp = self.format_timestamp();
        println!("{}{} {}", timestamp, level, message);
    }

    pub fn http(&self, ip: &str, method: &str, path: &str, status: Option<u16>, response_time: Option<u128>) {
        if !self.enable_request_logging {
            return;
        }

        let timestamp = self.format_timestamp();
        let ip_colored = ip.yellow();
        let request = format!("{} {}", method, path).cyan();
        
        if let (Some(status), Some(time)) = (status, response_time) {
            let status_colored = if status < 400 {
                format!("{}", status).green()
            } else {
                format!("{}", status).red()
            };
            println!("{}{} {} {} - {} in {} ms", 
                timestamp, 
                LogLevel::Http, 
                ip_colored, 
                request,
                status_colored,
                time
            );
        } else {
            println!("{}{} {} {}", timestamp, LogLevel::Http, ip_colored, request);
        }
    }

    pub fn info(&self, message: &str) {
        self.log(LogLevel::Info, message);
    }

    pub fn warn(&self, message: &str) {
        self.log(LogLevel::Warn, message);
    }

    pub fn error(&self, message: &str) {
        self.log(LogLevel::Error, message);
    }

    pub fn startup_info(&self, name: &str, version: &str, author: &str) {
        let startup_msg = format!("Starting {} v{} by {}", name.bold(), version.bold(), author.bold());
        self.info(&startup_msg);
    }

    pub fn server_info(&self, signature: &str, local_url: &str, network_url: Option<&str>) {
        self.info(&format!("Server: {}", signature.bold()));
        
        if std::env::var("NODE_ENV").as_deref() == Ok("production") || !atty::is(atty::Stream::Stdout) {
            let suffix = format!(" at {}", local_url);
            self.info(&format!("Accepting connections{}", suffix));
        } else {
            // Fancy boxed output
            let mut message = format!("{}", "Serving!".green().bold());
            
            if !local_url.is_empty() {
                let prefix = if network_url.is_some() { "- " } else { "" };
                let space = if network_url.is_some() { "    " } else { "  " };
                message += &format!("\n\n{}{}{}{}",
                    "Local:".bold(),
                    space,
                    prefix,
                    local_url.bright_cyan()
                );
            }
            
            if let Some(network) = network_url {
                message += &format!("\n{}  {}",
                    "- Network:".bold(),
                    network.bright_cyan()
                );
            }

            self.print_boxed(&message);
        }
    }

    pub fn print_boxed(&self, message: &str) {
        let lines: Vec<&str> = message.lines().collect();
        if lines.is_empty() {
            return;
        }

        // Calculate the width needed
        let max_width = lines.iter()
            .map(|line| strip_ansi_codes(line).len())
            .max()
            .unwrap_or(0);
        
        let box_width = max_width + 4; // 2 spaces padding on each side

        // Top border
        println!("┌{}┐", "─".repeat(box_width));
        
        // Empty line
        println!("│{}│", " ".repeat(box_width));
        
        // Content lines
        for line in lines {
            let stripped_len = strip_ansi_codes(line).len();
            let padding = " ".repeat((box_width - stripped_len) / 2);
            let right_padding = " ".repeat(box_width - stripped_len - padding.len());
            println!("│{}{}{}│", padding, line, right_padding);
        }
        
        // Empty line
        println!("│{}│", " ".repeat(box_width));
        
        // Bottom border
        println!("└{}┘", "─".repeat(box_width));
        println!(); // Extra newline after the box
    }

    pub fn shutdown_message(&self) {
        println!();
        self.info("Gracefully shutting down. Please wait...");
    }

    pub fn force_shutdown_message(&self) {
        println!();
        self.warn("Force-closing all open sockets...");
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self::new()
    }
}

// Helper function to strip ANSI color codes for width calculation
fn strip_ansi_codes(s: &str) -> String {
    let mut result = String::new();
    let mut in_escape = false;
    let mut chars = s.chars().peekable();
    
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            if chars.peek() == Some(&'[') {
                in_escape = true;
                continue;
            }
        }
        
        if in_escape {
            if ch.is_ascii_alphabetic() {
                in_escape = false;
            }
            continue;
        }
        
        result.push(ch);
    }
    
    result
}

// Global logger instance
use std::sync::OnceLock;
static GLOBAL_LOGGER: OnceLock<Logger> = OnceLock::new();

pub fn init_logger(enable_request_logging: bool, enable_timestamps: bool) {
    let _ = GLOBAL_LOGGER.set(
        Logger::new()
            .with_request_logging(enable_request_logging)
            .with_timestamps(enable_timestamps)
    );
}

pub fn get_logger() -> &'static Logger {
    GLOBAL_LOGGER.get().unwrap_or(&DEFAULT_LOGGER)
}

static DEFAULT_LOGGER: Logger = Logger {
    enable_request_logging: true,
    enable_timestamps: true,
};

// Convenience macros
#[macro_export]
macro_rules! log_http {
    ($ip:expr, $method:expr, $path:expr) => {
        $crate::logger::get_logger().http($ip, $method, $path, None, None)
    };
    ($ip:expr, $method:expr, $path:expr, $status:expr, $time:expr) => {
        $crate::logger::get_logger().http($ip, $method, $path, Some($status), Some($time))
    };
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        $crate::logger::get_logger().info(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        $crate::logger::get_logger().warn(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        $crate::logger::get_logger().error(&format!($($arg)*))
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logger_creation() {
        let logger = Logger::new();
        assert!(logger.enable_request_logging);
        assert!(logger.enable_timestamps);
    }

    #[test]
    fn test_logger_configuration() {
        let logger = Logger::new()
            .with_request_logging(false)
            .with_timestamps(false);
        
        assert!(!logger.enable_request_logging);
        assert!(!logger.enable_timestamps);
    }

    #[test]
    fn test_log_levels_display() {
        assert!(format!("{}", LogLevel::Http).contains("HTTP"));
        assert!(format!("{}", LogLevel::Info).contains("INFO"));
        assert!(format!("{}", LogLevel::Warn).contains("WARN"));
        assert!(format!("{}", LogLevel::Error).contains("ERROR"));
    }

    #[test]
    fn test_strip_ansi_codes() {
        let colored_text = "Hello".red().to_string();
        let stripped = strip_ansi_codes(&colored_text);
        assert_eq!(stripped, "Hello");
        
        let plain_text = "Plain text";
        let stripped_plain = strip_ansi_codes(plain_text);
        assert_eq!(stripped_plain, "Plain text");
    }

    #[test]
    fn test_timestamp_formatting() {
        let logger = Logger::new().with_timestamps(true);
        let timestamp = logger.format_timestamp();
        assert!(!timestamp.is_empty());
        
        let logger_no_timestamp = Logger::new().with_timestamps(false);
        let no_timestamp = logger_no_timestamp.format_timestamp();
        assert!(no_timestamp.is_empty());
    }
}