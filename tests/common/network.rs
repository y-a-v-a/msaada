//! Network testing utilities
//!
//! This module provides utilities for port management, network interface
//! detection, and concurrent connection testing.

use reqwest::Response;

use super::client::TestClient;

/// Network testing helpers
pub struct NetworkTestHelper;

impl NetworkTestHelper {
    /// Check if a port is available
    pub async fn is_port_available(port: u16) -> bool {
        use port_scanner::scan_port_addr;
        !scan_port_addr(std::net::SocketAddr::from(([127, 0, 0, 1], port)))
    }

    /// Get the next available port starting from a base port
    pub async fn get_available_port_from(
        start_port: u16,
    ) -> Result<u16, Box<dyn std::error::Error>> {
        for port in start_port..=65535 {
            if Self::is_port_available(port).await {
                return Ok(port);
            }
        }
        Err("No available ports found".into())
    }

    /// Test concurrent connections to a server
    pub async fn test_concurrent_connections(
        url: &str,
        num_connections: usize,
    ) -> Result<Vec<Result<Response, reqwest::Error>>, Box<dyn std::error::Error>> {
        let client = TestClient::new();
        let mut handles = Vec::new();

        for _ in 0..num_connections {
            let client = client.client.clone();
            let url = url.to_string();

            let handle = tokio::spawn(async move { client.get(&url).send().await });

            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            results.push(handle.await?);
        }

        Ok(results)
    }
}
