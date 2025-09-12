// src/clipboard.rs
// Clipboard integration for copying server URL

use clipboard::{ClipboardContext, ClipboardProvider};
use colored::*;
use std::error::Error;
use std::fmt;

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
        
        // Try to create clipboard context
        let mut ctx: ClipboardContext = match ClipboardProvider::new() {
            Ok(ctx) => ctx,
            Err(e) => {
                return Err(ClipboardError::NotAvailable(format!("{:?}", e)));
            }
        };
        
        // Try to copy text to clipboard
        match ctx.set_contents(text.to_string()) {
            Ok(_) => {
                let logger = crate::logger::get_logger();
                logger.info(&format!("📋 Copied to clipboard: {}", text.green()));
                Ok(())
            }
            Err(e) => {
                Err(ClipboardError::CopyFailed(format!("{:?}", e)))
            }
        }
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
        // Should not fail when disabled
        assert!(clipboard.copy_to_clipboard("test").is_ok());
        assert!(clipboard.copy_server_url("http://localhost:3000").is_ok());
    }
    
    #[test]
    fn test_clipboard_enabled() {
        // Note: This test might fail in CI environments without clipboard support
        let clipboard = ClipboardManager::new(true);
        
        // Try to copy but don't fail the test if clipboard is not available
        // (common in CI environments)
        match clipboard.copy_to_clipboard("test") {
            Ok(_) => println!("Clipboard copy succeeded"),
            Err(ClipboardError::NotAvailable(_)) => {
                println!("Clipboard not available (expected in CI)")
            }
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }
}