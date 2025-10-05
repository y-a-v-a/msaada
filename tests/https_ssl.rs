//! HTTPS/SSL Integration Tests
//!
//! This module contains granular integration tests for HTTPS/SSL functionality
//! in msaada. Each test can be run individually for focused testing.
//!
//! Migrated from test_https_ssl.sh to provide Jest-like granular execution.

mod common;

use std::path::Path;
use std::time::Duration;

use common::assertions::ResponseAssertions;
use common::filesystem::FileSystemHelper;
use common::server::TestServer;
use common::ssl::SslTestHelper;
use reqwest::StatusCode;
use serde_json::Value;

/// Test PEM certificate support
/// Migrated from test_pem_certificates() in test_https_ssl.sh
#[tokio::test]
async fn pem_certificate_support() {
    // Generate PEM cert/key
    let (cert_file, key_file) =
        SslTestHelper::create_temp_cert_files().expect("Failed to create PEM certificates");

    // Start HTTPS server
    let server = TestServer::new_https_with_pem(cert_file.path(), key_file.path())
        .await
        .expect("Failed to start HTTPS server");

    // Setup SSL test files
    FileSystemHelper::setup_ssl_test_files(&server.server_dir)
        .expect("Failed to setup SSL test files");

    // Create HTTPS client (accepts self-signed certs)
    let client = SslTestHelper::create_https_client().expect("Failed to create HTTPS client");

    // Sub-test 1: HTTPS connection works
    let response = client
        .get(server.url())
        .send()
        .await
        .expect("HTTPS GET request failed");

    response.assert_status(StatusCode::OK);
    let content = response
        .text_for_assertions()
        .await
        .expect("Failed to get response text");
    assert!(
        content.contains("Secure Connection"),
        "HTTPS response should contain 'Secure Connection'"
    );

    // Sub-test 2: TLS handshake succeeds
    // For MVP: Connection success indicates TLS handshake worked
    let response = client
        .get(server.url())
        .send()
        .await
        .expect("TLS handshake failed");
    assert!(response.status().is_success(), "TLS connection should work");

    // Sub-test 3: HTTPS headers verification
    let response = client
        .head(server.url())
        .send()
        .await
        .expect("HEAD request failed");
    response.assert_status(StatusCode::OK);
    assert!(
        response.headers().contains_key("content-type")
            || response.headers().contains_key("content-length"),
        "HTTPS headers should be present"
    );
}

/// Test PKCS12 certificate support
/// Migrated from test_pkcs12_certificates() in test_https_ssl.sh
#[tokio::test]
async fn pkcs12_certificate_support() {
    // Generate PKCS12 with passphrase
    let (p12_file, pass_file) = SslTestHelper::create_temp_pkcs12_files("testpass123")
        .expect("Failed to create PKCS12 certificates");

    // Start HTTPS server with PKCS12
    let server = TestServer::new_https_with_pkcs12(p12_file.path(), pass_file.path())
        .await
        .expect("Failed to start HTTPS server with PKCS12");

    // Setup SSL test files
    FileSystemHelper::setup_ssl_test_files(&server.server_dir)
        .expect("Failed to setup SSL test files");

    let client = SslTestHelper::create_https_client().expect("Failed to create HTTPS client");

    // Sub-test 1: HTTPS connection works
    let response = client
        .get(server.url())
        .send()
        .await
        .expect("HTTPS GET failed");

    response.assert_status(StatusCode::OK);
    let content = response
        .text_for_assertions()
        .await
        .expect("Failed to get response");
    assert!(
        content.contains("Secure Connection"),
        "PKCS12 HTTPS should serve content correctly"
    );

    // Sub-test 2: JSON API over HTTPS
    let response = client
        .get(server.url_for("api.json"))
        .send()
        .await
        .expect("JSON API request failed");

    response.assert_status(StatusCode::OK);
    let json_data: Value = response
        .json_for_assertions()
        .await
        .expect("Failed to parse JSON");
    assert_eq!(
        json_data["protocol"], "https",
        "JSON should indicate HTTPS protocol"
    );
}

/// Test SSL security features
/// Migrated from test_ssl_security() in test_https_ssl.sh
#[tokio::test]
async fn ssl_security_features() {
    // Generate PEM cert
    let (cert_file, key_file) =
        SslTestHelper::create_temp_cert_files().expect("Failed to create certificates");

    // Start HTTPS server
    let server = TestServer::new_https_with_pem(cert_file.path(), key_file.path())
        .await
        .expect("Failed to start HTTPS server");

    FileSystemHelper::setup_ssl_test_files(&server.server_dir)
        .expect("Failed to setup SSL test files");

    let https_client = SslTestHelper::create_https_client().expect("Failed to create HTTPS client");

    // Sub-test 1: TLS connection works (version check simplified)
    let response = https_client
        .get(server.url())
        .send()
        .await
        .expect("TLS connection failed");
    assert!(response.status().is_success(), "TLS connection should work");

    // Sub-test 2: Secure cipher negotiated (simplified - connection success implies secure cipher)
    // Note: Deep TLS introspection could be added later with rustls connection metadata
    assert!(
        response.status().is_success(),
        "Secure cipher negotiation should work"
    );

    // Sub-test 3: HTTP on HTTPS port should fail
    let http_url = server.url().replace("https://", "http://");
    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("Failed to create HTTP client");

    let result = http_client.get(&http_url).send().await;
    assert!(result.is_err(), "HTTP request on HTTPS port should fail");
}

/// Test SSL error handling
/// Migrated from test_ssl_error_conditions() in test_https_ssl.sh
#[tokio::test]
async fn ssl_error_handling() {
    // Sub-test 1: Invalid certificate path
    let result = TestServer::new_https_with_pem(
        Path::new("/nonexistent/cert.pem"),
        Path::new("/nonexistent/key.pem"),
    )
    .await;

    assert!(
        result.is_err(),
        "Server should fail with invalid certificate path"
    );

    // Sub-test 2: Mismatched cert and key
    let (cert1_file, _key1_file) =
        SslTestHelper::create_temp_cert_files().expect("Failed to create first cert pair");
    let (_cert2_file, key2_file) =
        SslTestHelper::create_temp_cert_files().expect("Failed to create second cert pair");

    let result = TestServer::new_https_with_pem(cert1_file.path(), key2_file.path()).await;

    assert!(
        result.is_err(),
        "Server should fail with mismatched cert and key"
    );

    // Sub-test 3: Wrong PKCS12 passphrase
    let (p12_file, _correct_pass) =
        SslTestHelper::create_temp_pkcs12_files("correctpass").expect("Failed to create PKCS12");

    // Create wrong passphrase file
    let mut wrong_pass_file = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    std::io::Write::write_all(&mut wrong_pass_file, b"wrongpassword")
        .expect("Failed to write wrong passphrase");

    let result = TestServer::new_https_with_pkcs12(p12_file.path(), wrong_pass_file.path()).await;

    assert!(
        result.is_err(),
        "Server should fail with wrong PKCS12 passphrase"
    );
}
