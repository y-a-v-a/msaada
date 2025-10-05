//! Integration tests for the common test utilities

mod common;

use common::*;
use serde_json::json;

#[tokio::test]
async fn test_port_availability() {
    let port = NetworkTestHelper::get_available_port_from(3200).await.unwrap();
    assert!(port >= 3200);
    assert!(NetworkTestHelper::is_port_available(port).await);
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

#[tokio::test]
async fn test_http_client_creation() {
    let _client = TestClient::new();
    // Just verify client creation doesn't panic - no assertion needed
}

#[test]
fn test_file_system_helpers() {
    use std::fs;
    let temp_dir = tempfile::TempDir::new().unwrap();

    // Test HTML file creation
    let html_path = FileSystemHelper::create_html_file(
        temp_dir.path(),
        "test.html",
        "Test Title",
        "<p>Test content</p>"
    ).unwrap();

    let content = fs::read_to_string(&html_path).unwrap();
    assert!(content.contains("Test Title"));
    assert!(content.contains("<p>Test content</p>"));

    // Test JSON file creation
    let json_path = FileSystemHelper::create_json_file(
        temp_dir.path(),
        "test.json",
        &json!({"test": true, "count": 42})
    ).unwrap();

    let json_content = fs::read_to_string(&json_path).unwrap();
    assert!(json_content.contains("\"test\": true"));
    assert!(json_content.contains("\"count\": 42"));
}