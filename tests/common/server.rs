//! Test server management utilities
//!
//! This module provides the TestServer struct and related functionality for
//! managing msaada server instances during testing.

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU16, Ordering};
use std::time::Duration;

use reqwest::Client;
use tempfile::TempDir;
use tokio::time::{sleep, timeout};

use super::network::NetworkTestHelper;

/// Global port counter to avoid port conflicts in parallel tests
static PORT_COUNTER: AtomicU16 = AtomicU16::new(3100);

/// Test server configuration and lifecycle management
pub struct TestServer {
    pub process: Child,
    pub port: u16,
    pub base_url: String,
    pub temp_dir: TempDir,
    pub server_dir: PathBuf,
}

impl TestServer {
    /// Start a new msaada test server with automatic port selection
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Self::new_with_options(None, None).await
    }

    /// Start a new msaada test server with custom options
    pub async fn new_with_options(
        custom_dir: Option<PathBuf>,
        extra_args: Option<Vec<String>>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let port = get_available_port().await?;
        let temp_dir = TempDir::new()?;
        let server_dir = custom_dir.unwrap_or_else(|| temp_dir.path().to_path_buf());

        // Create basic directory structure
        std::fs::create_dir_all(&server_dir)?;

        // Get path to msaada binary
        let binary_path = get_msaada_binary_path()?;

        // Build command arguments
        let mut args = vec![
            "--port".to_string(),
            port.to_string(),
            "--dir".to_string(),
            server_dir.to_string_lossy().to_string(),
        ];

        if let Some(extra) = extra_args {
            args.extend(extra);
        }

        // Start the server process
        let process = Command::new(binary_path)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // Wait for server to be ready
        let base_url = format!("http://localhost:{}", port);
        wait_for_server_ready(&base_url).await?;

        Ok(TestServer {
            process,
            port,
            base_url,
            temp_dir,
            server_dir,
        })
    }

    /// Get the server's base URL
    pub fn url(&self) -> &str {
        &self.base_url
    }

    /// Get a URL for a specific path
    pub fn url_for(&self, path: &str) -> String {
        let path = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("/{}", path)
        };
        format!("{}{}", self.base_url, path)
    }

    /// Stop the server gracefully
    pub fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.process.kill()?;
        self.process.wait()?;
        Ok(())
    }

    /// Start HTTPS server with PEM certificates
    pub async fn new_https_with_pem(
        cert_file: &std::path::Path,
        key_file: &std::path::Path,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let extra_args = vec![
            "--ssl-cert".to_string(),
            cert_file.to_string_lossy().to_string(),
            "--ssl-key".to_string(),
            key_file.to_string_lossy().to_string(),
        ];

        let port = get_available_port().await?;
        let temp_dir = TempDir::new()?;
        let server_dir = temp_dir.path().to_path_buf();

        // Create basic directory structure
        std::fs::create_dir_all(&server_dir)?;

        // Get path to msaada binary
        let binary_path = get_msaada_binary_path()?;

        // Build command arguments
        let mut args = vec![
            "--port".to_string(),
            port.to_string(),
            "--dir".to_string(),
            server_dir.to_string_lossy().to_string(),
        ];
        args.extend(extra_args);

        // Start the server process
        let process = Command::new(binary_path)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // Wait for HTTPS server to be ready
        let base_url = format!("https://localhost:{}", port);
        wait_for_https_server_ready(&base_url).await?;

        Ok(TestServer {
            process,
            port,
            base_url,
            temp_dir,
            server_dir,
        })
    }

    /// Start HTTPS server with PKCS12 certificate
    pub async fn new_https_with_pkcs12(
        p12_file: &std::path::Path,
        pass_file: &std::path::Path,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let extra_args = vec![
            "--ssl-cert".to_string(),
            p12_file.to_string_lossy().to_string(),
            "--ssl-pass".to_string(),
            pass_file.to_string_lossy().to_string(),
        ];

        let port = get_available_port().await?;
        let temp_dir = TempDir::new()?;
        let server_dir = temp_dir.path().to_path_buf();

        // Create basic directory structure
        std::fs::create_dir_all(&server_dir)?;

        // Get path to msaada binary
        let binary_path = get_msaada_binary_path()?;

        // Build command arguments
        let mut args = vec![
            "--port".to_string(),
            port.to_string(),
            "--dir".to_string(),
            server_dir.to_string_lossy().to_string(),
        ];
        args.extend(extra_args);

        // Start the server process
        let process = Command::new(binary_path)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // Wait for HTTPS server to be ready
        let base_url = format!("https://localhost:{}", port);
        wait_for_https_server_ready(&base_url).await?;

        Ok(TestServer {
            process,
            port,
            base_url,
            temp_dir,
            server_dir,
        })
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

/// Helper function to get next available port
async fn get_available_port() -> Result<u16, Box<dyn std::error::Error>> {
    let start_port = PORT_COUNTER.fetch_add(1, Ordering::SeqCst);
    NetworkTestHelper::get_available_port_from(start_port).await
}

/// Wait for server to be ready by polling the health endpoint
async fn wait_for_server_ready(base_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let max_attempts = 50;
    let delay = Duration::from_millis(100);

    for _ in 0..max_attempts {
        match timeout(Duration::from_secs(5), client.get(base_url).send()).await {
            Ok(Ok(_)) => return Ok(()),
            _ => sleep(delay).await,
        }
    }

    Err(format!("Server at {} did not become ready in time", base_url).into())
}

/// Wait for HTTPS server to be ready by polling with SSL-aware client
async fn wait_for_https_server_ready(base_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Use client that accepts self-signed certificates
    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_secs(5))
        .build()?;

    let max_attempts = 50;
    let delay = Duration::from_millis(100);

    for _ in 0..max_attempts {
        match timeout(Duration::from_secs(5), client.get(base_url).send()).await {
            Ok(Ok(_)) => return Ok(()),
            _ => sleep(delay).await,
        }
    }

    Err(format!("HTTPS server at {} did not become ready in time", base_url).into())
}

/// Get the path to the msaada binary
fn get_msaada_binary_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    // Try release build first, then debug build
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")?;
    let release_path = PathBuf::from(&manifest_dir).join("target/release/msaada");
    let debug_path = PathBuf::from(&manifest_dir).join("target/debug/msaada");

    if release_path.exists() {
        Ok(release_path)
    } else if debug_path.exists() {
        Ok(debug_path)
    } else {
        Err("msaada binary not found. Run 'cargo build' or 'cargo build --release' first.".into())
    }
}