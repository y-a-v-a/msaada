//! Common test utilities for msaada integration tests
//!
//! This module provides shared functionality for testing the msaada HTTP server,
//! organized into focused sub-modules for better maintainability.

#![allow(dead_code)] // Test utilities will be used by integration tests
#![allow(unused_imports)] // Some re-exports may not be used in all test modules

// Sub-modules
pub mod assertions;
pub mod client;
pub mod filesystem;
pub mod network;
pub mod server;
pub mod ssl;

// Re-export commonly used types and functions for convenience
pub use client::TestClient;
pub use filesystem::{FileSystemHelper, TestStructure};
pub use network::NetworkTestHelper;
pub use ssl::SslTestHelper;

// Re-export external types that are commonly used in tests
pub use serde_json::json;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_port_availability() {
        // This is a basic test to ensure the port checking works
        let port = NetworkTestHelper::get_available_port_from(3200)
            .await
            .unwrap();
        assert!(port >= 3200);
    }

    #[test]
    fn test_test_structure_builder() {
        let structure = TestStructure::new()
            .add_html_file("index.html", "Test Page", "<h1>Hello World</h1>")
            .add_text_file("README.txt", "This is a test")
            .add_json_file("config.json", json!({"key": "value"}));

        assert_eq!(structure.files.len(), 3);
    }

    #[test]
    fn test_ssl_certificate_generation() {
        let result = SslTestHelper::generate_test_certificate();
        assert!(result.is_ok());

        let (cert_pem, key_pem) = result.unwrap();
        assert!(cert_pem.contains("BEGIN CERTIFICATE"));
        assert!(key_pem.contains("BEGIN PRIVATE KEY"));
    }
}
