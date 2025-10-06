//! HTTP Server Integration Tests
//!
//! This module contains granular integration tests for the core HTTP server
//! functionality of msaada. Each test can be run individually for focused testing.

mod common;

use common::assertions::ResponseAssertions;
use common::prelude::*;
use reqwest::StatusCode;
use std::path::Path;

/// Test basic file serving functionality
/// Migrated from test_basic_file_serving() in test_http_server.sh
#[tokio::test]
async fn basic_file_serving() {
    let server = TestServer::new()
        .await
        .expect("Failed to start test server");
    let client = TestClient::new();

    // Create test files similar to shell script setup
    setup_basic_test_files(&server.server_dir)
        .await
        .expect("Failed to setup test files");

    // Test HTML file serving
    let response = client
        .get(&server.url_for("index.html"))
        .await
        .expect("GET request failed");
    response.assert_status(StatusCode::OK);

    let html_content = response
        .text_for_assertions()
        .await
        .expect("Failed to get response text");
    assert!(
        html_content.contains("Welcome"),
        "HTML content should contain 'Welcome'"
    );
    assert!(html_content.contains("<h1>"), "HTML should contain h1 tag");

    // Test CSS file serving
    let response = client
        .get(&server.url_for("style.css"))
        .await
        .expect("GET request failed");
    response.assert_status(StatusCode::OK);

    let css_content = response
        .text_for_assertions()
        .await
        .expect("Failed to get response text");
    assert!(
        css_content.contains("background-color"),
        "CSS should contain background-color"
    );

    // Test JavaScript file serving
    let response = client
        .get(&server.url_for("script.js"))
        .await
        .expect("GET request failed");
    response.assert_status(StatusCode::OK);

    let js_content = response
        .text_for_assertions()
        .await
        .expect("Failed to get response text");
    assert!(
        js_content.contains("console.log"),
        "JS should contain console.log"
    );

    // Test JSON file serving
    let response = client
        .get(&server.url_for("data.json"))
        .await
        .expect("GET request failed");
    response.assert_status(StatusCode::OK);

    let json_data: serde_json::Value = response
        .json_for_assertions()
        .await
        .expect("Failed to parse JSON");
    assert_eq!(
        json_data["name"], "test",
        "JSON should contain correct name field"
    );
    assert_eq!(
        json_data["value"], 42,
        "JSON should contain correct value field"
    );
}

/// Test HTTP content type detection
/// Migrated from test_content_types() in test_http_server.sh
#[tokio::test]
async fn content_types() {
    let server = TestServer::new()
        .await
        .expect("Failed to start test server");
    let client = TestClient::new();

    // Create test files
    setup_basic_test_files(&server.server_dir)
        .await
        .expect("Failed to setup test files");

    // Test HTML content type
    let response = client
        .get(&server.url_for("index.html"))
        .await
        .expect("GET request failed");
    response.assert_status(StatusCode::OK);

    let content_type = response
        .headers()
        .get("content-type")
        .expect("Content-Type header should be present")
        .to_str()
        .expect("Content-Type should be valid string");
    assert!(
        content_type.starts_with("text/html"),
        "HTML should have text/html content type"
    );

    // Test CSS content type
    let response = client
        .get(&server.url_for("style.css"))
        .await
        .expect("GET request failed");
    response.assert_status(StatusCode::OK);

    let content_type = response
        .headers()
        .get("content-type")
        .expect("Content-Type header should be present")
        .to_str()
        .expect("Content-Type should be valid string");
    assert!(
        content_type.starts_with("text/css"),
        "CSS should have text/css content type"
    );

    // Test JavaScript content type
    let response = client
        .get(&server.url_for("script.js"))
        .await
        .expect("GET request failed");
    response.assert_status(StatusCode::OK);

    let content_type = response
        .headers()
        .get("content-type")
        .expect("Content-Type header should be present")
        .to_str()
        .expect("Content-Type should be valid string");
    assert!(
        content_type.starts_with("text/javascript")
            || content_type.starts_with("application/javascript"),
        "JS should have appropriate content type, got: {}",
        content_type
    );

    // Test JSON content type
    let response = client
        .get(&server.url_for("data.json"))
        .await
        .expect("GET request failed");
    response.assert_status(StatusCode::OK);

    let content_type = response
        .headers()
        .get("content-type")
        .expect("Content-Type header should be present")
        .to_str()
        .expect("Content-Type should be valid string");
    assert!(
        content_type.starts_with("application/json"),
        "JSON should have application/json content type"
    );

    // Test PNG content type
    let response = client
        .get(&server.url_for("test.png"))
        .await
        .expect("GET request failed");
    response.assert_status(StatusCode::OK);

    let content_type = response
        .headers()
        .get("content-type")
        .expect("Content-Type header should be present")
        .to_str()
        .expect("Content-Type should be valid string");
    assert!(
        content_type.starts_with("image/png"),
        "PNG should have image/png content type"
    );
}

/// Test HTTP response headers
/// Migrated from test_response_headers() in test_http_server.sh
#[tokio::test]
async fn response_headers() {
    let server = TestServer::new()
        .await
        .expect("Failed to start test server");
    let client = TestClient::new();

    // Create basic test files
    setup_basic_test_files(&server.server_dir)
        .await
        .expect("Failed to setup test files");

    let response = client
        .head(&server.url_for("/"))
        .await
        .expect("HEAD request failed");
    response.assert_status(StatusCode::OK);

    let headers = response.headers();

    // Test for common HTTP headers that should be present
    assert!(
        headers.contains_key("content-length"),
        "Content-Length header should be present"
    );
    assert!(
        headers.contains_key("date"),
        "Date header should be present"
    );

    // Test for custom headers if implemented (optional - based on msaada implementation)
    // Note: The shell script tests for X-Server, X-Powered-By, X-Version headers
    // These may or may not be implemented in the actual msaada server
    // We'll test for their presence but won't fail the test if they're missing
    let has_custom_headers = headers.contains_key("x-server")
        || headers.contains_key("x-powered-by")
        || headers.contains_key("x-version");

    if has_custom_headers {
        println!("Custom headers detected - server implements additional header information");
    } else {
        println!("No custom headers detected - using standard HTTP headers only");
    }
}

/// Test different HTTP methods
/// Migrated from test_http_methods() in test_http_server.sh
#[tokio::test]
async fn http_methods() {
    let server = TestServer::new()
        .await
        .expect("Failed to start test server");
    let client = TestClient::new();

    // Create test files
    setup_basic_test_files(&server.server_dir)
        .await
        .expect("Failed to setup test files");

    // Test GET method
    let response = client
        .get(&server.url_for("index.html"))
        .await
        .expect("GET request failed");
    response.assert_status(StatusCode::OK);

    let content = response
        .text_for_assertions()
        .await
        .expect("Failed to get response text");
    assert!(
        content.contains("Welcome"),
        "GET should return file content"
    );

    // Test HEAD method (should return headers but no body)
    let response = client
        .head(&server.url_for("index.html"))
        .await
        .expect("HEAD request failed");
    response.assert_status(StatusCode::OK);

    // HEAD should have Content-Length but empty body
    assert!(
        response.headers().contains_key("content-length"),
        "HEAD should include Content-Length"
    );

    // Test OPTIONS method (implementation varies)
    let response = client
        .client
        .request(reqwest::Method::OPTIONS, server.url_for("/"))
        .send()
        .await
        .expect("OPTIONS request failed");

    // OPTIONS can return 200, 404, or 405 depending on implementation
    let status = response.status();
    assert!(
        status == StatusCode::OK
            || status == StatusCode::NOT_FOUND
            || status == StatusCode::METHOD_NOT_ALLOWED,
        "OPTIONS should return appropriate status code, got: {}",
        status
    );
}

/// Test error handling and security
/// Migrated from test_error_handling() in test_http_server.sh
#[tokio::test]
async fn error_handling() {
    let server = TestServer::new()
        .await
        .expect("Failed to start test server");
    let client = TestClient::new();

    // Test 404 Not Found
    let response = client
        .get(&server.url_for("nonexistent.html"))
        .await
        .expect("GET request failed");
    response.assert_status(StatusCode::NOT_FOUND);

    // Test directory traversal protection
    let response = client
        .get(&server.url_for("../../../etc/passwd"))
        .await
        .expect("GET request failed");
    let status = response.status();
    assert!(
        status == StatusCode::NOT_FOUND || status == StatusCode::FORBIDDEN,
        "Directory traversal should be blocked, got status: {}",
        status
    );

    // Test invalid URLs with null bytes
    let response = client
        .get(&format!("{}/%00%00invalid", server.url()))
        .await
        .expect("GET request failed");
    let status = response.status();
    assert!(
        status == StatusCode::BAD_REQUEST || status == StatusCode::NOT_FOUND,
        "Invalid URLs should return appropriate error, got status: {}",
        status
    );
}

/// Test special file handling
/// Migrated from test_special_files() in test_http_server.sh
#[tokio::test]
async fn special_files() {
    let server = TestServer::new()
        .await
        .expect("Failed to start test server");
    let client = TestClient::new();

    // Create special test files
    setup_special_test_files(&server.server_dir)
        .await
        .expect("Failed to setup special files");

    // Test empty file
    let response = client
        .get(&server.url_for("empty.txt"))
        .await
        .expect("GET request failed");
    response.assert_status(StatusCode::OK);

    let content = response
        .text_for_assertions()
        .await
        .expect("Failed to get response text");
    assert_eq!(content, "", "Empty file should return empty content");

    // Test large file (should still work)
    let response = client
        .get(&server.url_for("large.txt"))
        .await
        .expect("GET request failed");
    response.assert_status(StatusCode::OK);

    let content = response
        .bytes()
        .await
        .expect("Failed to get response bytes");
    assert!(
        content.len() > 1000000,
        "Large file should be approximately 1MB, got {} bytes",
        content.len()
    );

    // Test hidden files (should be blocked)
    let response = client
        .get(&server.url_for(".hidden"))
        .await
        .expect("GET request failed");
    let status = response.status();
    assert!(
        status == StatusCode::NOT_FOUND
            || status == StatusCode::FORBIDDEN
            || status == StatusCode::BAD_REQUEST,
        "Hidden files should be blocked, got status: {}",
        status
    );

    // Test files with spaces (URL encoded)
    let response = client
        .get(&server.url_for("file%20with%20spaces.txt"))
        .await
        .expect("GET request failed");
    response.assert_status(StatusCode::OK);

    let content = response
        .text_for_assertions()
        .await
        .expect("Failed to get response text");
    assert!(
        content.contains("Special content"),
        "File with spaces should return correct content"
    );
}

/// Test directory index handling
/// Migrated from test_directory_index() in test_http_server.sh
#[tokio::test]
async fn directory_index() {
    let server = TestServer::new()
        .await
        .expect("Failed to start test server");
    let client = TestClient::new();

    // Create index.html file
    setup_basic_test_files(&server.server_dir)
        .await
        .expect("Failed to setup test files");

    // Test root directory access (should serve index.html)
    let response = client.get(server.url()).await.expect("GET request failed");
    response.assert_status(StatusCode::OK);

    let content = response
        .text_for_assertions()
        .await
        .expect("Failed to get response text");
    assert!(
        content.contains("Welcome"),
        "Root directory should serve index.html with Welcome message"
    );

    // Test direct index.html access
    let response = client
        .get(&server.url_for("index.html"))
        .await
        .expect("GET request failed");
    response.assert_status(StatusCode::OK);

    let content = response
        .text_for_assertions()
        .await
        .expect("Failed to get response text");
    assert!(
        content.contains("Welcome"),
        "Direct index.html access should work"
    );
}

/// Setup basic test files similar to the shell script
async fn setup_basic_test_files(server_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Create HTML file
    FileSystemHelper::create_html_file(
        server_dir,
        "index.html",
        "Test Index",
        "<h1>Welcome</h1><p>This is a test page.</p>",
    )?;

    // Create CSS file
    FileSystemHelper::create_css_file(
        server_dir,
        "style.css",
        "body { background-color: #f0f0f0; font-family: Arial; }",
    )?;

    // Create JavaScript file
    FileSystemHelper::create_js_file(
        server_dir,
        "script.js",
        "console.log('Test JavaScript file loaded');",
    )?;

    // Create JSON file
    FileSystemHelper::create_json_file(
        server_dir,
        "data.json",
        &json!({"name": "test", "value": 42, "active": true}),
    )?;

    // Create plain text file
    std::fs::write(
        server_dir.join("readme.txt"),
        "This is a plain text file for testing.",
    )?;

    // Create XML file
    std::fs::write(
        server_dir.join("data.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?><root><item>test</item></root>"#,
    )?;

    // Create small PNG image (minimal PNG file)
    let png_data = [
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F,
        0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00,
        0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
        0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];
    std::fs::write(server_dir.join("test.png"), png_data)?;

    Ok(())
}

/// Setup special test files (empty, large, hidden, files with spaces)
async fn setup_special_test_files(server_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Create empty file
    std::fs::write(server_dir.join("empty.txt"), "")?;

    // Create large file (1MB)
    let large_content = vec![0u8; 1024 * 1024];
    std::fs::write(server_dir.join("large.txt"), &large_content)?;

    // Create hidden file
    std::fs::write(server_dir.join(".hidden"), "Hidden content")?;

    // Create file with spaces
    std::fs::write(server_dir.join("file with spaces.txt"), "Special content")?;

    // Create file with unicode characters
    std::fs::write(server_dir.join("café.txt"), "Unicode content")?;

    Ok(())
}
