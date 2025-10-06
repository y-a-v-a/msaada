//! POST Request Handling Integration Tests
//!
//! This module contains granular integration tests for POST request handling
//! in msaada. Each test can be run individually for focused testing.
//!
//! Migrated from test_post_enhanced.sh to provide Jest-like granular execution.

mod common;

use std::collections::HashMap;

use common::assertions::ResponseAssertions;
use common::filesystem::FileSystemHelper;
use common::prelude::*;
use reqwest::StatusCode;
use serde_json::{json, Value};

/// Test JSON POST request handling
/// Migrated from test_json_post() in test_post_enhanced.sh
#[tokio::test]
async fn json_post_handling() {
    let server = TestServer::new()
        .await
        .expect("Failed to start test server");

    // Setup POST test files
    FileSystemHelper::setup_post_test_files(&server.server_dir)
        .expect("Failed to setup POST test files");

    let client = TestClient::new();

    // Test 1: Simple JSON POST
    let simple_json = json!({"name": "test", "value": 42});
    let response = client
        .post_json(&server.url_for("/api/simple"), &simple_json)
        .await
        .expect("Simple JSON POST failed");

    response.assert_status(StatusCode::OK);
    let json_response: Value = response
        .json_for_assertions()
        .await
        .expect("Failed to parse JSON response");

    // Note: Server returns path without leading slash
    let path_value = json_response["path"]
        .as_str()
        .expect("path should be string");
    assert!(
        path_value.ends_with("api/simple"),
        "JSON POST path should match, got: {}",
        path_value
    );
    assert!(
        json_response["json_data"].is_object(),
        "JSON POST should have json_data field"
    );
    assert_eq!(
        json_response["json_data"]["name"], "test",
        "JSON POST data should be echoed back"
    );
    assert_eq!(
        json_response["json_data"]["value"], 42,
        "JSON POST numeric value should be preserved"
    );

    // Test 2: Nested JSON POST
    let nested_json = json!({
        "user": {"name": "John", "age": 30},
        "data": {"items": [1, 2, 3], "active": true}
    });
    let response = client
        .post_json(&server.url_for("/api/nested"), &nested_json)
        .await
        .expect("Nested JSON POST failed");

    response.assert_status(StatusCode::OK);
    let json_response: Value = response
        .json_for_assertions()
        .await
        .expect("Failed to parse nested JSON response");

    assert_eq!(
        json_response["json_data"]["user"]["name"], "John",
        "Nested JSON structure should be preserved"
    );
    assert_eq!(
        json_response["json_data"]["data"]["active"], true,
        "Nested boolean values should be preserved"
    );
    assert_eq!(
        json_response["json_data"]["data"]["items"],
        json!([1, 2, 3]),
        "Nested arrays should be preserved"
    );

    // Test 3: Empty JSON POST
    let empty_json = json!({});
    let response = client
        .post_json(&server.url_for("/api/empty"), &empty_json)
        .await
        .expect("Empty JSON POST failed");

    response.assert_status(StatusCode::OK);
    let json_response: Value = response
        .json_for_assertions()
        .await
        .expect("Failed to parse empty JSON response");

    assert!(
        json_response["path"].is_string(),
        "Empty JSON response should have path field"
    );

    // Test 4: Invalid JSON (should be handled gracefully)
    // We send it as text since we can't construct invalid JSON with serde
    let response = client
        .post_text(&server.url_for("/api/invalid"), r#"{"invalid": json}"#)
        .await
        .expect("Invalid JSON POST failed");

    response.assert_status(StatusCode::OK);
    let json_response: Value = response
        .json_for_assertions()
        .await
        .expect("Failed to parse invalid JSON response");

    // Should be handled as text data since it's invalid JSON
    assert!(
        json_response["text_data"].is_string() || json_response["content_type"].is_string(),
        "Invalid JSON should be handled gracefully"
    );
}

/// Test form data POST request handling
/// Migrated from test_form_post() in test_post_enhanced.sh
#[tokio::test]
async fn form_post_handling() {
    let server = TestServer::new()
        .await
        .expect("Failed to start test server");

    // Setup POST test files
    FileSystemHelper::setup_post_test_files(&server.server_dir)
        .expect("Failed to setup POST test files");

    let client = TestClient::new();

    // Test 1: Simple form data POST
    let mut form_data = HashMap::new();
    form_data.insert("name".to_string(), "test".to_string());
    form_data.insert("value".to_string(), "42".to_string());
    form_data.insert("active".to_string(), "true".to_string());

    let response = client
        .post_form(&server.url_for("/api/form"), &form_data)
        .await
        .expect("Simple form POST failed");

    response.assert_status(StatusCode::OK);
    let json_response: Value = response
        .json_for_assertions()
        .await
        .expect("Failed to parse form response");

    assert!(
        json_response["form_data"].is_object(),
        "Form POST should have form_data field"
    );
    let path_value = json_response["path"]
        .as_str()
        .expect("path should be string");
    assert!(
        path_value.ends_with("api/form"),
        "Form POST path should match, got: {}",
        path_value
    );

    // Test 2: Form data with special characters
    let mut special_form = HashMap::new();
    special_form.insert("message".to_string(), "Hello World!".to_string());
    special_form.insert("symbols".to_string(), "@#$%".to_string());

    let response = client
        .post_form(&server.url_for("/api/special"), &special_form)
        .await
        .expect("Special chars form POST failed");

    response.assert_status(StatusCode::OK);
    let json_response: Value = response
        .json_for_assertions()
        .await
        .expect("Failed to parse special chars form response");

    assert!(
        json_response.is_object(),
        "Special characters form should be handled correctly"
    );

    // Test 3: Empty form data
    let empty_form = HashMap::new();

    let response = client
        .post_form(&server.url_for("/api/empty-form"), &empty_form)
        .await
        .expect("Empty form POST failed");

    response.assert_status(StatusCode::OK);
    let json_response: Value = response
        .json_for_assertions()
        .await
        .expect("Failed to parse empty form response");

    assert!(
        json_response["path"].is_string(),
        "Empty form response should have path field"
    );
}

/// Test multipart file upload handling
/// Migrated from test_multipart_upload() in test_post_enhanced.sh
#[tokio::test]
async fn multipart_file_upload() {
    let server = TestServer::new()
        .await
        .expect("Failed to start test server");

    // Setup POST test files
    let test_files = FileSystemHelper::setup_post_test_files(&server.server_dir)
        .expect("Failed to setup POST test files");

    let client = TestClient::new();

    // Test 1: Single file upload
    let sample_txt_content =
        std::fs::read(&test_files.sample_txt).expect("Failed to read sample.txt");
    let form = reqwest::multipart::Form::new().part(
        "document",
        reqwest::multipart::Part::bytes(sample_txt_content)
            .file_name("sample.txt")
            .mime_str("text/plain")
            .expect("Failed to set MIME type"),
    );

    let response = client
        .post_multipart(&server.url_for("/api/upload"), form)
        .await
        .expect("Single file upload failed");

    response.assert_status(StatusCode::OK);
    let json_response: Value = response
        .json_for_assertions()
        .await
        .expect("Failed to parse upload response");

    assert!(
        json_response["files"].is_array(),
        "Upload response should have files array"
    );
    let response_text =
        serde_json::to_string(&json_response).expect("Failed to serialize response");
    assert!(
        response_text.contains("sample.txt"),
        "Upload response should contain filename"
    );

    // Test 2: Image file upload
    let sample_png_content =
        std::fs::read(&test_files.sample_png).expect("Failed to read sample.png");
    let form = reqwest::multipart::Form::new().part(
        "image",
        reqwest::multipart::Part::bytes(sample_png_content)
            .file_name("sample.png")
            .mime_str("image/png")
            .expect("Failed to set MIME type"),
    );

    let response = client
        .post_multipart(&server.url_for("/api/upload-image"), form)
        .await
        .expect("Image upload failed");

    response.assert_status(StatusCode::OK);
    let json_response: Value = response
        .json_for_assertions()
        .await
        .expect("Failed to parse image upload response");

    let response_text =
        serde_json::to_string(&json_response).expect("Failed to serialize response");
    assert!(
        response_text.contains("sample.png"),
        "Image upload response should contain filename"
    );

    // Test 3: Multipart with form fields and file
    let sample_json_content =
        std::fs::read(&test_files.sample_json).expect("Failed to read sample.json");
    let form = reqwest::multipart::Form::new()
        .text("name", "test_upload")
        .text("description", "A test file upload")
        .part(
            "file",
            reqwest::multipart::Part::bytes(sample_json_content)
                .file_name("sample.json")
                .mime_str("application/json")
                .expect("Failed to set MIME type"),
        );

    let response = client
        .post_multipart(&server.url_for("/api/upload-with-fields"), form)
        .await
        .expect("Multipart with fields upload failed");

    response.assert_status(StatusCode::OK);
    let json_response: Value = response
        .json_for_assertions()
        .await
        .expect("Failed to parse multipart response");

    assert!(
        json_response["form_data"].is_object() || json_response["files"].is_array(),
        "Multipart response should have form_data or files"
    );

    // Test 4: Large file upload
    let large_file_content =
        std::fs::read(&test_files.large_file).expect("Failed to read large_file.bin");
    let form = reqwest::multipart::Form::new().part(
        "largefile",
        reqwest::multipart::Part::bytes(large_file_content)
            .file_name("large_file.bin")
            .mime_str("application/octet-stream")
            .expect("Failed to set MIME type"),
    );

    let response = client
        .post_multipart(&server.url_for("/api/upload-large"), form)
        .await
        .expect("Large file upload failed");

    response.assert_status(StatusCode::OK);
    let json_response: Value = response
        .json_for_assertions()
        .await
        .expect("Failed to parse large file response");

    let response_text =
        serde_json::to_string(&json_response).expect("Failed to serialize response");
    assert!(
        response_text.contains("large_file.bin"),
        "Large file upload response should contain filename"
    );
}

/// Test plain text POST request handling
/// Migrated from test_text_post() in test_post_enhanced.sh
#[tokio::test]
async fn text_post_handling() {
    let server = TestServer::new()
        .await
        .expect("Failed to start test server");

    // Setup POST test files
    FileSystemHelper::setup_post_test_files(&server.server_dir)
        .expect("Failed to setup POST test files");

    let client = TestClient::new();

    // Test 1: Simple text POST
    let simple_text = "This is a simple text message.";
    let response = client
        .post_text(&server.url_for("/api/text"), simple_text)
        .await
        .expect("Simple text POST failed");

    response.assert_status(StatusCode::OK);
    let json_response: Value = response
        .json_for_assertions()
        .await
        .expect("Failed to parse text response");

    assert!(
        json_response["text_data"].is_string(),
        "Text POST should have text_data field"
    );
    assert_eq!(
        json_response["text_data"], simple_text,
        "Text content should be echoed back"
    );

    // Test 2: Multiline text POST
    let multiline_text = "Line 1\nLine 2\nLine 3 with special chars: @#$%";
    let response = client
        .post_text(&server.url_for("/api/multiline"), multiline_text)
        .await
        .expect("Multiline text POST failed");

    response.assert_status(StatusCode::OK);
    let json_response: Value = response
        .json_for_assertions()
        .await
        .expect("Failed to parse multiline response");

    assert!(
        json_response["text_data"].is_string(),
        "Multiline text POST should have text_data field"
    );

    // Test 3: Empty text POST
    let empty_text = "";
    let response = client
        .post_text(&server.url_for("/api/empty-text"), empty_text)
        .await
        .expect("Empty text POST failed");

    response.assert_status(StatusCode::OK);
    let json_response: Value = response
        .json_for_assertions()
        .await
        .expect("Failed to parse empty text response");

    assert!(
        json_response.is_object(),
        "Empty text response should be valid JSON"
    );
}

/// Test binary data POST request handling
/// Migrated from test_binary_post() in test_post_enhanced.sh
#[tokio::test]
async fn binary_post_handling() {
    let server = TestServer::new()
        .await
        .expect("Failed to start test server");

    // Setup POST test files
    let test_files = FileSystemHelper::setup_post_test_files(&server.server_dir)
        .expect("Failed to setup POST test files");

    let client = TestClient::new();

    // Test 1: Binary data POST with octet-stream
    let binary_data = std::fs::read(&test_files.sample_png).expect("Failed to read sample.png");
    let response = client
        .post_binary(
            &server.url_for("/api/binary"),
            binary_data,
            "application/octet-stream",
        )
        .await
        .expect("Binary POST failed");

    response.assert_status(StatusCode::OK);
    let json_response: Value = response
        .json_for_assertions()
        .await
        .expect("Failed to parse binary response");

    assert!(
        json_response["content_type"].is_string(),
        "Binary POST response should have content_type field"
    );
    let content_type = json_response["content_type"]
        .as_str()
        .expect("content_type should be string");
    assert!(
        content_type.contains("application/octet-stream"),
        "Binary POST content type should be octet-stream, got: {}",
        content_type
    );

    // Test 2: Custom binary content type (PDF)
    let binary_data = std::fs::read(&test_files.large_file).expect("Failed to read large_file.bin");
    let response = client
        .post_binary(&server.url_for("/api/pdf"), binary_data, "application/pdf")
        .await
        .expect("PDF binary POST failed");

    response.assert_status(StatusCode::OK);
    let json_response: Value = response
        .json_for_assertions()
        .await
        .expect("Failed to parse PDF response");

    assert!(
        json_response["content_type"].is_string(),
        "PDF POST response should have content_type field"
    );
    let content_type = json_response["content_type"]
        .as_str()
        .expect("content_type should be string");
    assert!(
        content_type.contains("application/pdf"),
        "PDF POST content type should be application/pdf, got: {}",
        content_type
    );
}

/// Test POST response format consistency
/// Migrated from test_response_format() in test_post_enhanced.sh
#[tokio::test]
async fn post_response_format() {
    let server = TestServer::new()
        .await
        .expect("Failed to start test server");

    // Setup POST test files
    FileSystemHelper::setup_post_test_files(&server.server_dir)
        .expect("Failed to setup POST test files");

    let client = TestClient::new();

    // Test 1: JSON POST response format
    let json_data = json!({"test": true});
    let response = client
        .post_json(&server.url_for("/api/test-json"), &json_data)
        .await
        .expect("JSON POST failed");

    response.assert_status(StatusCode::OK);
    let json_response: Value = response
        .json_for_assertions()
        .await
        .expect("Failed to parse JSON response");

    assert!(
        json_response["path"].is_string(),
        "Response should have path field"
    );
    assert!(
        json_response["content_type"].is_string(),
        "Response should have content_type field"
    );
    let path_value = json_response["path"]
        .as_str()
        .expect("path should be string");
    assert!(
        path_value.ends_with("api/test-json"),
        "Path field should match request path, got: {}",
        path_value
    );

    // Test 2: Form POST response format
    let mut form_data = HashMap::new();
    form_data.insert("name".to_string(), "test".to_string());

    let response = client
        .post_form(&server.url_for("/api/test-form"), &form_data)
        .await
        .expect("Form POST failed");

    response.assert_status(StatusCode::OK);
    let json_response: Value = response
        .json_for_assertions()
        .await
        .expect("Failed to parse form response");

    assert!(
        json_response["path"].is_string(),
        "Form response should have path field"
    );
    assert!(
        json_response["content_type"].is_string(),
        "Form response should have content_type field"
    );
    let path_value = json_response["path"]
        .as_str()
        .expect("path should be string");
    assert!(
        path_value.ends_with("api/test-form"),
        "Path field should match request path, got: {}",
        path_value
    );

    // Test 3: Text POST response format
    let text_data = "Test text";
    let response = client
        .post_text(&server.url_for("/api/test-text"), text_data)
        .await
        .expect("Text POST failed");

    response.assert_status(StatusCode::OK);
    let json_response: Value = response
        .json_for_assertions()
        .await
        .expect("Failed to parse text response");

    assert!(
        json_response["path"].is_string(),
        "Text response should have path field"
    );
    assert!(
        json_response["content_type"].is_string(),
        "Text response should have content_type field"
    );
    let path_value = json_response["path"]
        .as_str()
        .expect("path should be string");
    assert!(
        path_value.ends_with("api/test-text"),
        "Path field should match request path, got: {}",
        path_value
    );
}
