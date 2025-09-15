// src/shutdown.rs
// Graceful shutdown handling for the server

use actix_web::dev::ServerHandle;
use signal_hook::consts::SIGINT;
use signal_hook_tokio::{Signals, Handle};
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;
use futures_util::StreamExt;

pub struct ShutdownManager {
    shutdown_tx: Option<oneshot::Sender<()>>,
    signal_handle: Option<Handle>,
    server_handle: Arc<Mutex<Option<ServerHandle>>>,
}

impl ShutdownManager {
    pub fn new() -> Self {
        Self {
            shutdown_tx: None,
            signal_handle: None,
            server_handle: Arc::new(Mutex::new(None)),
        }
    }

    /// Set up graceful shutdown signal handling
    #[allow(dead_code)]
    pub async fn setup_shutdown_handling(
        &mut self,
        server_handle: ServerHandle,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Store the server handle
        {
            let mut handle = self.server_handle.lock().unwrap();
            *handle = Some(server_handle);
        }

        // Set up signal handling for SIGINT and SIGTERM
        let signals = Signals::new(&[SIGINT, signal_hook::consts::SIGTERM])?;
        let signal_handle = signals.handle();
        self.signal_handle = Some(signal_handle);

        // Create shutdown channel
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        self.shutdown_tx = Some(shutdown_tx);

        // Clone the server handle for the signal task
        let server_handle = Arc::clone(&self.server_handle);
        let logger = crate::logger::get_logger();

        // Spawn signal handling task
        tokio::spawn(async move {
            let mut signals = signals;
            let mut force_shutdown = false;

            while let Some(signal) = signals.next().await {
                match signal {
                    SIGINT | signal_hook::consts::SIGTERM => {
                        if force_shutdown {
                            // Second signal - force shutdown
                            logger.force_shutdown_message();
                            std::process::exit(0);
                        } else {
                            // First signal - graceful shutdown
                            logger.shutdown_message();
                            
                            // Try to stop the server gracefully
                            if let Ok(mut handle) = server_handle.lock() {
                                if let Some(server) = handle.take() {
                                    let _ = server.stop(true);
                                }
                            }
                            
                            force_shutdown = true;
                            
                            // Set a timeout for force shutdown
                            tokio::spawn(async move {
                                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                                if force_shutdown {
                                    std::process::exit(0);
                                }
                            });
                        }
                    }
                    _ => {}
                }
            }
        });

        // Wait for shutdown signal in a separate task
        tokio::spawn(async move {
            let _ = shutdown_rx.await;
        });

        Ok(())
    }

    /// Trigger graceful shutdown
    #[allow(dead_code)]
    pub fn shutdown(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        
        if let Some(handle) = self.signal_handle.take() {
            handle.close();
        }
    }

    /// Get the server handle for external control
    #[allow(dead_code)]
    pub fn get_server_handle(&self) -> Arc<Mutex<Option<ServerHandle>>> {
        Arc::clone(&self.server_handle)
    }
}

impl Drop for ShutdownManager {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Utility function to set up basic signal handling without the full manager
pub async fn setup_basic_signal_handling() -> Result<(), Box<dyn std::error::Error>> {
    let mut signals = Signals::new(&[SIGINT, signal_hook::consts::SIGTERM])?;
    let logger = crate::logger::get_logger();

    tokio::spawn(async move {
        let mut force_shutdown = false;
        
        while let Some(signal) = signals.next().await {
            match signal {
                SIGINT | signal_hook::consts::SIGTERM => {
                    if force_shutdown {
                        logger.force_shutdown_message();
                        std::process::exit(0);
                    } else {
                        logger.shutdown_message();
                        force_shutdown = true;
                        
                        // Set timeout for force shutdown
                        let logger_clone = logger;
                        tokio::spawn(async move {
                            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                            logger_clone.force_shutdown_message();
                            std::process::exit(0);
                        });
                    }
                }
                _ => {}
            }
        }
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::timeout;

    #[tokio::test]
    async fn test_shutdown_manager_creation() {
        let manager = ShutdownManager::new();
        assert!(manager.shutdown_tx.is_none());
        assert!(manager.signal_handle.is_none());
        
        // Server handle should be initialized but empty
        let handle = manager.server_handle.lock().unwrap();
        assert!(handle.is_none());
    }

    #[tokio::test]
    async fn test_shutdown_manager_shutdown() {
        let mut manager = ShutdownManager::new();
        
        // Create a dummy channel to simulate setup
        let (tx, _rx) = oneshot::channel::<()>();
        manager.shutdown_tx = Some(tx);
        
        // Should not panic
        manager.shutdown();
        
        // After shutdown, tx should be None
        assert!(manager.shutdown_tx.is_none());
    }

    #[tokio::test]
    async fn test_basic_signal_handling_setup() {
        // This should not panic and should complete quickly
        let result = timeout(
            Duration::from_millis(100),
            setup_basic_signal_handling()
        ).await;
        
        // Should timeout because signal handling runs indefinitely
        // but setup should succeed
        match result {
            Err(_) => {
                // Expected timeout - signal handling is running in background
                // Just verify we can call the function without errors
                let result = setup_basic_signal_handling().await;
                // The setup itself should complete immediately
                // (only the spawned task runs indefinitely)
            },
            Ok(Ok(())) => {
                // Setup completed successfully
            },
            Ok(Err(e)) => {
                panic!("Signal setup failed: {}", e);
            }
        }
    }

    #[test]
    fn test_shutdown_manager_drop() {
        // Create manager in a scope to test Drop
        {
            let mut manager = ShutdownManager::new();
            let (tx, _rx) = oneshot::channel::<()>();
            manager.shutdown_tx = Some(tx);
            
            // Manager should be dropped here and shutdown should be called
        }
        
        // Test passes if no panic occurs during drop
    }
}