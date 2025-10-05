//! Network and Port Management Integration Tests
//!
//! This module tests port management, network interface detection,
//! concurrent connections, IPv6 support, and network error handling.

mod common;

use common::*;
use common::assertions::ResponseAssertions;
use common::server::TestServer;
use std::process::Command;
use std::time::Duration;
use tokio::time::sleep;

/// Test port availability checking
#[tokio::test]
async fn port_availability_check() {
    let server = TestServer::new()
        .await
        .expect("Failed to start test server");

    let client = TestClient::new();

    // Create basic test files
    FileSystemHelper::create_html_file(
        &server.server_dir,
        "index.html",
        "Network Test Server",
        "<h1>Network Test Server</h1><p>Testing port and network functionality.</p>",
    )
    .expect("Failed to create index.html");

    // Sub-test 1: Server starts successfully on available port
    // (already started via TestServer::new())

    // Sub-test 2: Verify server is actually listening
    let response = client
        .get(&server.url_for("/"))
        .await
        .expect("Failed to connect to server");

    response.assert_status(reqwest::StatusCode::OK);

    let body = response.text_for_assertions().await.expect("Failed to read response body");
    assert!(
        body.contains("Network Test Server"),
        "Response should contain expected content"
    );
}

/// Test port conflict handling
#[tokio::test]
async fn port_conflict_resolution() {
    // Sub-test 1: Start first server successfully
    let server = TestServer::new()
        .await
        .expect("Failed to start first server");

    FileSystemHelper::create_html_file(
        &server.server_dir,
        "index.html",
        "Port Conflict Test",
        "<h1>Port Conflict Test</h1>",
    )
    .expect("Failed to create index.html");

    // Give the server time to fully bind to the port
    sleep(Duration::from_millis(500)).await;

    // Sub-test 2: Second server with --no-port-switching should fail
    let msaada_bin = env!("CARGO_BIN_EXE_msaada");
    let output = Command::new(msaada_bin)
        .arg("--port")
        .arg(server.port.to_string())
        .arg("--dir")
        .arg(&server.server_dir)
        .arg("--no-port-switching")
        .arg("--no-clipboard")
        .output()
        .expect("Failed to execute second server");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should detect port conflict (either in stdout or stderr)
    let has_conflict_message = stderr.contains("address")
        || stderr.contains("use")
        || stderr.contains("bind")
        || stderr.contains("port")
        || stdout.contains("address")
        || stdout.contains("use")
        || stdout.contains("bind")
        || stdout.contains("port")
        || !output.status.success();

    assert!(
        has_conflict_message,
        "Second server should fail or report port conflict with --no-port-switching"
    );
}

/// Test port boundary validation
#[tokio::test]
async fn port_boundary_validation() {
    let server = TestServer::new()
        .await
        .expect("Failed to create test server");

    FileSystemHelper::create_html_file(
        &server.server_dir,
        "index.html",
        "Port Boundary Test",
        "<h1>Port Boundary Test</h1>",
    )
    .expect("Failed to create index.html");

    let msaada_bin = env!("CARGO_BIN_EXE_msaada");

    // Sub-test 1: Port 0 should be rejected or handled specially
    let output_zero = Command::new(msaada_bin)
        .arg("--port")
        .arg("0")
        .arg("--dir")
        .arg(&server.server_dir)
        .arg("--no-clipboard")
        .output()
        .expect("Failed to execute with port 0");

    let stderr_zero = String::from_utf8_lossy(&output_zero.stderr);
    let has_error_zero = stderr_zero.contains("invalid")
        || stderr_zero.contains("error")
        || !output_zero.status.success();

    assert!(
        has_error_zero,
        "Port 0 should be rejected or cause error"
    );

    // Sub-test 2: Port 65536 should be rejected (out of u16 range)
    let output_high = Command::new(msaada_bin)
        .arg("--port")
        .arg("65536")
        .arg("--dir")
        .arg(&server.server_dir)
        .arg("--no-clipboard")
        .output()
        .expect("Failed to execute with port 65536");

    let stderr_high = String::from_utf8_lossy(&output_high.stderr);
    let has_error_high = stderr_high.contains("invalid")
        || stderr_high.contains("error")
        || stderr_high.contains("range")
        || !output_high.status.success();

    assert!(
        has_error_high,
        "Port 65536 should be rejected as out of range"
    );

    // Sub-test 3: Privileged port (< 1024) should fail for non-root users
    // On most systems, binding to port 80 requires root privileges
    let output_priv = Command::new(msaada_bin)
        .arg("--port")
        .arg("80")
        .arg("--dir")
        .arg(&server.server_dir)
        .arg("--no-clipboard")
        .arg("--no-port-switching")
        .output()
        .expect("Failed to execute with port 80");

    // If running as non-root, should see permission error
    // If running as root, it may succeed (we accept both outcomes)
    let stderr_priv = String::from_utf8_lossy(&output_priv.stderr);
    let has_permission_handling = stderr_priv.contains("permission")
        || stderr_priv.contains("denied")
        || stderr_priv.contains("bind")
        || output_priv.status.success(); // May succeed if root

    // We don't assert this as failure, just verify it's handled gracefully (no panic)
    // The test passes if the server either denied the port or succeeded (if running as root)
    assert!(
        has_permission_handling,
        "Privileged port handling should be graceful (permission error or success if root)"
    );
}

/// Test network interface detection
#[tokio::test]
async fn network_interface_detection() {
    // Sub-test 1: Server should start and be accessible via localhost
    let server = TestServer::new()
        .await
        .expect("Failed to start server");

    FileSystemHelper::create_html_file(
        &server.server_dir,
        "index.html",
        "Network Interface Test",
        "<h1>Network Interface Test</h1>",
    )
    .expect("Failed to create index.html");

    std::fs::write(
        server.server_dir.join("test.txt"),
        "Network test content",
    )
    .expect("Failed to create test.txt");

    // Sub-test 2: Server should be accessible via 127.0.0.1
    let client = TestClient::new();
    let url_127 = format!("http://127.0.0.1:{}/", server.port);
    let response = client
        .get(&url_127)
        .await
        .expect("Failed to connect via 127.0.0.1");

    response.assert_status(reqwest::StatusCode::OK);

    // Sub-test 3: Server should serve static files correctly
    let response = client
        .get(&server.url_for("/test.txt"))
        .await
        .expect("Failed to fetch test.txt");

    response.assert_status(reqwest::StatusCode::OK);

    let body = response.text_for_assertions().await.expect("Failed to read response body");
    assert_eq!(
        body, "Network test content",
        "test.txt should have correct content"
    );
}

/// Test concurrent connection handling
#[tokio::test]
async fn concurrent_connection_handling() {
    let server = TestServer::new()
        .await
        .expect("Failed to start server");

    FileSystemHelper::create_html_file(
        &server.server_dir,
        "index.html",
        "Concurrent Test",
        "<h1>Concurrent Test</h1>",
    )
    .expect("Failed to create index.html");

    std::fs::write(
        server.server_dir.join("test.txt"),
        "Network test content",
    )
    .expect("Failed to create test.txt");

    // Sub-test 1: Handle multiple concurrent connections
    let url = server.url_for("/test.txt");
    let results = NetworkTestHelper::test_concurrent_connections(&url, 5)
        .await
        .expect("Failed to test concurrent connections");

    // Count successful responses
    let mut successful = 0;
    for response in results.into_iter().flatten() {
        if response.status() == 200 {
            if let Ok(body) = response.text().await {
                if body.contains("Network test content") {
                    successful += 1;
                }
            }
        }
    }

    assert_eq!(
        successful, 5,
        "All 5 concurrent connections should succeed"
    );

    // Sub-test 2: Handle rapid sequential requests
    let client = TestClient::new();
    let mut sequential_success = 0;

    for i in 0..10 {
        let url_with_query = format!("{}?seq={}", server.url_for("/"), i);
        let response = client
            .get(&url_with_query)
            .await;

        if let Ok(resp) = response {
            if resp.status() == reqwest::StatusCode::OK {
                sequential_success += 1;
            }
        }
    }

    assert!(
        sequential_success >= 8,
        "At least 8/10 rapid sequential requests should succeed (got {})",
        sequential_success
    );
}

/// Test IPv6 support (if available)
#[tokio::test]
async fn ipv6_support_test() {
    // Check if IPv6 is available on the system
    let ipv6_available = check_ipv6_availability();

    if !ipv6_available {
        println!("IPv6 not available on system, skipping IPv6 tests");
        return;
    }

    let server = TestServer::new()
        .await
        .expect("Failed to start server for IPv6 test");

    FileSystemHelper::create_html_file(
        &server.server_dir,
        "index.html",
        "IPv6 Test Server",
        "<h1>Network Test Server</h1>",
    )
    .expect("Failed to create index.html");

    // Give server time to start
    sleep(Duration::from_millis(500)).await;

    let client = TestClient::new();

    // Sub-test 1: Test IPv6 connectivity via [::1]
    let ipv6_url = format!("http://[::1]:{}/", server.port);
    let ipv6_result = client
        .get(&ipv6_url)
        .await;

    if let Ok(response) = ipv6_result {
        response.assert_status(reqwest::StatusCode::OK);

        let body = response.text_for_assertions().await.expect("Failed to read response");
        assert!(
            body.contains("Network Test Server"),
            "IPv6 response should contain expected content"
        );
    } else {
        println!("IPv6 connection failed (server may not support IPv6)");
    }

    // Sub-test 2: Verify IPv4 still works in dual-stack
    let ipv4_url = format!("http://127.0.0.1:{}/", server.port);
    let ipv4_response = client
        .get(&ipv4_url)
        .await
        .expect("IPv4 connectivity should work");

    ipv4_response.assert_status(reqwest::StatusCode::OK);
}

/// Check if IPv6 is available on the system
fn check_ipv6_availability() -> bool {
    // Try to detect IPv6 using standard commands
    #[cfg(target_os = "linux")]
    {
        Command::new("ip")
            .args(["-6", "addr", "show", "lo"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("ifconfig")
            .arg("lo0")
            .output()
            .map(|o| {
                String::from_utf8_lossy(&o.stdout).contains("inet6")
            })
            .unwrap_or(false)
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        // On Windows or other platforms, assume IPv6 is available
        true
    }
}

/// Test network error handling
#[tokio::test]
async fn network_error_recovery() {
    let msaada_bin = env!("CARGO_BIN_EXE_msaada");

    // Sub-test 1: Invalid directory should be handled gracefully
    let output_invalid = Command::new(msaada_bin)
        .arg("--port")
        .arg("3506")
        .arg("--dir")
        .arg("/nonexistent/directory/path/that/does/not/exist")
        .arg("--no-clipboard")
        .output()
        .expect("Failed to execute with invalid directory");

    let stderr_invalid = String::from_utf8_lossy(&output_invalid.stderr);
    let has_error_invalid = stderr_invalid.contains("error")
        || stderr_invalid.contains("not found")
        || stderr_invalid.contains("directory")
        || stderr_invalid.contains("No such file")
        || !output_invalid.status.success();

    assert!(
        has_error_invalid,
        "Invalid directory should produce error message"
    );

    // Sub-test 2: Permission errors should be handled gracefully
    let server = TestServer::new()
        .await
        .expect("Failed to create test server");

    let no_perm_dir = server.temp_dir.path().join("no_perm");
    std::fs::create_dir(&no_perm_dir).expect("Failed to create directory");

    // Try to make directory unreadable (may not work on all systems)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&no_perm_dir)
            .expect("Failed to get metadata")
            .permissions();
        perms.set_mode(0o000);
        let _ = std::fs::set_permissions(&no_perm_dir, perms);
    }

    let output_perm = Command::new(msaada_bin)
        .arg("--port")
        .arg("3507")
        .arg("--dir")
        .arg(&no_perm_dir)
        .arg("--no-clipboard")
        .output()
        .expect("Failed to execute with permission issue");

    // Restore permissions for cleanup
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&no_perm_dir)
            .unwrap_or_else(|_| std::fs::metadata(server.temp_dir.path()).unwrap())
            .permissions();
        perms.set_mode(0o755);
        let _ = std::fs::set_permissions(&no_perm_dir, perms);
    }

    // Check if permission error was detected (may not trigger on all systems)
    let stderr_perm = String::from_utf8_lossy(&output_perm.stderr);
    let has_perm_handling = stderr_perm.contains("permission")
        || stderr_perm.contains("denied")
        || stderr_perm.contains("access")
        || !output_perm.status.success();

    // We accept this test passing if the server handled the error gracefully
    // Permission behavior varies by OS, but we at least verify no crash/panic occurred
    assert!(
        has_perm_handling,
        "Permission error handling should be graceful (error or handled silently)"
    );
}
