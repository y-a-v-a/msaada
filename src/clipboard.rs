// src/clipboard.rs
// Clipboard integration for copying server URL without external crates

use colored::*;
use std::error::Error;
use std::fmt;
use std::io::{self, Write};
use std::process::{Command, Stdio};

#[derive(Debug)]
pub enum ClipboardError {
    NotAvailable(String),
    CopyFailed(String),
}

impl fmt::Display for ClipboardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClipboardError::NotAvailable(msg) => write!(f, "Clipboard not available: {}", msg),
            ClipboardError::CopyFailed(msg) => write!(f, "Failed to copy to clipboard: {}", msg),
        }
    }
}

impl Error for ClipboardError {}

#[derive(Debug, Clone, Copy)]
struct ClipboardCommand {
    program: &'static str,
    args: &'static [&'static str],
    description: &'static str,
}

#[derive(Debug)]
enum CommandError {
    NotFound,
    Failed(String),
}

impl ClipboardCommand {
    fn execute(&self, text: &str) -> Result<(), CommandError> {
        let mut child = Command::new(self.program)
            .args(self.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|err| match err.kind() {
                io::ErrorKind::NotFound => CommandError::NotFound,
                _ => CommandError::Failed(err.to_string()),
            })?;

        {
            let stdin = child
                .stdin
                .as_mut()
                .ok_or_else(|| CommandError::Failed("failed to open stdin".to_string()))?;
            stdin
                .write_all(text.as_bytes())
                .map_err(|err| CommandError::Failed(err.to_string()))?;
        }

        let status = child
            .wait()
            .map_err(|err| CommandError::Failed(err.to_string()))?;
        if status.success() {
            Ok(())
        } else {
            let message = match status.code() {
                Some(code) => format!("exited with status {code}"),
                None => "process terminated by signal".to_string(),
            };
            Err(CommandError::Failed(message))
        }
    }
}

#[cfg(all(unix, not(any(target_os = "macos", target_os = "android"))))]
fn command_candidates() -> &'static [ClipboardCommand] {
    &[
        ClipboardCommand {
            program: "wl-copy",
            args: &[],
            description: "wl-copy (Wayland)",
        },
        ClipboardCommand {
            program: "xclip",
            args: &["-selection", "clipboard"],
            description: "xclip (X11)",
        },
        ClipboardCommand {
            program: "xsel",
            args: &["--clipboard", "--input"],
            description: "xsel (X11)",
        },
    ]
}

#[cfg(target_os = "macos")]
fn command_candidates() -> &'static [ClipboardCommand] {
    &[ClipboardCommand {
        program: "pbcopy",
        args: &[],
        description: "pbcopy (macOS)",
    }]
}

#[cfg(target_os = "windows")]
fn command_candidates() -> &'static [ClipboardCommand] {
    &[ClipboardCommand {
        program: "cmd",
        args: &["/C", "clip"],
        description: "Windows clip command",
    }]
}

#[cfg(not(any(
    all(unix, not(any(target_os = "macos", target_os = "android"))),
    target_os = "macos",
    target_os = "windows"
)))]
fn command_candidates() -> &'static [ClipboardCommand] {
    &[]
}

pub struct ClipboardManager {
    enabled: bool,
}

impl ClipboardManager {
    pub fn new(enabled: bool) -> Self {
        ClipboardManager { enabled }
    }

    /// Copy text to clipboard if enabled
    pub fn copy_to_clipboard(&self, text: &str) -> Result<(), ClipboardError> {
        if !self.enabled {
            return Ok(());
        }

        let commands = command_candidates();
        if commands.is_empty() {
            return Err(ClipboardError::NotAvailable(
                "Clipboard copying is not supported on this platform".to_string(),
            ));
        }

        let mut not_found_reasons = Vec::new();
        let mut failure_reasons = Vec::new();

        for command in commands {
            match command.execute(text) {
                Ok(()) => {
                    let logger = crate::logger::get_logger();
                    logger.info(&format!(
                        "📋 Copied to clipboard using {}: {}",
                        command.description,
                        text.green()
                    ));
                    return Ok(());
                }
                Err(CommandError::NotFound) => {
                    not_found_reasons.push(format!("{} not found", command.description));
                }
                Err(CommandError::Failed(reason)) => {
                    failure_reasons.push(format!("{} failed: {}", command.description, reason));
                }
            }
        }

        if !failure_reasons.is_empty() {
            if !not_found_reasons.is_empty() {
                failure_reasons.extend(not_found_reasons);
            }
            return Err(ClipboardError::CopyFailed(failure_reasons.join("; ")));
        }

        let message = if not_found_reasons.is_empty() {
            "No clipboard command available".to_string()
        } else {
            not_found_reasons.join("; ")
        };
        Err(ClipboardError::NotAvailable(message))
    }

    /// Copy server URL to clipboard with nice formatting
    pub fn copy_server_url(&self, url: &str) -> Result<(), ClipboardError> {
        if !self.enabled {
            return Ok(());
        }

        self.copy_to_clipboard(url)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clipboard_disabled() {
        let clipboard = ClipboardManager::new(false);
        assert!(clipboard.copy_to_clipboard("test").is_ok());
        assert!(clipboard.copy_server_url("http://localhost:3000").is_ok());
    }

    #[test]
    fn test_clipboard_enabled() {
        let clipboard = ClipboardManager::new(true);

        match clipboard.copy_to_clipboard("test") {
            Ok(_) => println!("Clipboard copy succeeded"),
            Err(ClipboardError::NotAvailable(_)) => {
                println!("Clipboard command not available (expected in CI)")
            }
            Err(ClipboardError::CopyFailed(e)) => {
                panic!("Unexpected clipboard failure: {}", e)
            }
        }
    }
}
