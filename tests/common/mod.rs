//! Common test utilities for msaada integration tests
//!
//! This module provides shared functionality for testing the msaada HTTP server,
//! organized into focused sub-modules for better maintainability.

// Sub-modules
pub mod assertions;
pub mod client;
pub mod filesystem;
pub mod network;
pub mod server;
pub mod ssl;

/// Shared imports for the majority of integration tests.
///
/// Pulling in this prelude keeps call sites concise while allowing us to prune or
/// extend the underlying helpers without exposing the entire module tree.
pub mod prelude {
    pub use super::client::TestClient;
    pub use super::filesystem::{FileSystemHelper, TestStructure};
    pub use super::network::NetworkTestHelper;
    pub use super::server::TestServer;
    pub use super::ssl::SslTestHelper;
    pub use serde_json::json;
}

#[cfg(test)]
mod tests {
    use super::prelude::{json, NetworkTestHelper, SslTestHelper, TestStructure};

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
