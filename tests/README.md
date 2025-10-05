# Msaada Testing Framework

This directory contains comprehensive Rust integration tests for the Msaada HTTP server with granular test execution, parallel testing, and full IDE integration.

## Quick Start

Run all tests:
```bash
cargo test
```

Run specific test suites (by file):
```bash
cargo test --test http_server      # All tests in http_server.rs
cargo test --test config           # All tests in config.rs
cargo test --test post_requests    # All tests in post_requests.rs
cargo test --test https_ssl        # All tests in https_ssl.rs
cargo test --test advanced_features # All tests in advanced_features.rs
cargo test --test network_ports    # All tests in network_ports.rs
```

Run tests by name pattern:
```bash
cargo test file_serving     # Any test with "file_serving" in name
cargo test json             # Any test with "json" in name
```

Run with detailed output:
```bash
RUST_BACKTRACE=1 cargo test --verbose
```

## Test Suites

### Core Test Suites

#### http_server.rs (7 tests)
Tests core HTTP server functionality:
- Static file serving and content types
- HTTP methods (GET, HEAD, OPTIONS)
- Response headers and caching
- Error handling (404, 403, etc.)
- Directory indexing and special files

#### https_ssl.rs (4 tests)
Tests HTTPS/SSL integration:
- PEM certificate support (separate cert/key files)
- PKCS12 certificate support (combined .p12/.pfx files)
- Certificate validation and error handling
- Security features and SSL handshakes

#### config.rs (6 tests)
Tests configuration file system:
- serve.json, now.json, package.json precedence
- Custom configuration paths
- JSON schema validation
- Configuration error handling

#### post_requests.rs (6 tests)
Enhanced POST request testing:
- JSON POST requests (simple, nested, invalid)
- Form-encoded data (simple, special chars, empty)
- Multipart file uploads (single, multiple, with fields)
- Plain text and binary data handling
- Response format consistency

#### advanced_features.rs (8 tests)
Advanced web server features:
- CORS functionality and headers
- Gzip compression (enabled/disabled)
- SPA mode (Single Page Application routing)
- Caching headers (ETag, Last-Modified)
- Symlinks support
- Directory listing
- Configuration-based URL rewrites and headers
- Graceful shutdown handling

#### network_ports.rs (7 tests)
Network and port management:
- Port availability checking
- Port conflict handling and auto-switching
- Port boundary cases and validation
- Network interface detection
- Concurrent connections
- IPv6 support
- Network error handling

### Test Utilities

#### common/ directory
Shared utility modules for all test suites:
- **server.rs** - Server lifecycle management with TestServer
- **filesystem.rs** - File/directory creation and validation helpers
- **ssl.rs** - SSL certificate generation for HTTPS testing
- **network.rs** - Port management and network utilities
- **assertions.rs** - Response validation trait extensions
- **client.rs** - Enhanced HTTP client wrappers

## Test Directory Structure

```
tests/
├── README.md                      # This documentation
├── http_server.rs                 # Core HTTP tests (7 tests)
├── https_ssl.rs                   # HTTPS/SSL tests (4 tests)
├── config.rs                      # Configuration tests (6 tests)
├── post_requests.rs               # POST request tests (6 tests)
├── advanced_features.rs           # Advanced feature tests (8 tests)
├── network_ports.rs               # Network/port tests (7 tests)
├── test_utilities.rs              # Utility tests (2 tests)
├── todo-rust-integration.md       # Migration tracking document
└── common/                        # Shared test utilities
    ├── mod.rs                     # Common module entry
    ├── server.rs                  # Test server management
    ├── filesystem.rs              # File system helpers
    ├── ssl.rs                     # SSL/TLS utilities
    ├── network.rs                 # Network testing helpers
    ├── assertions.rs              # Response assertions
    └── client.rs                  # HTTP client helpers
```

## Test Features

### Comprehensive Coverage
- **41 integration tests** covering all msaada features
- **50+ sub-tests** for granular validation
- **Parallel execution** by default (isolated ports/temp dirs)
- **Zero flaky tests** with proper async/await handling
- **Rich error context** with detailed failure messages
- **Cross-platform** compatibility (Ubuntu, macOS, Windows)

### Robust Testing
- **Automatic resource management** with RAII and Drop trait
- **Dynamic port allocation** to prevent conflicts
- **Self-signed certificate generation** for HTTPS testing
- **Concurrent connection testing** for reliability
- **Type-safe assertions** for correctness
- **Graceful cleanup** with temporary directories

### Developer Friendly
- **IDE integration** - Run tests from VS Code, IntelliJ, etc.
- **Debugging support** - Set breakpoints in tests
- **Filtered execution** - Run specific tests by name
- **Watch mode** - `cargo watch -x test`
- **Coverage reports** - `cargo tarpaulin`
- **Clear diagnostics** - RUST_BACKTRACE for stack traces

## Requirements

### System Requirements
- **Rust 1.70+** (MSRV for async/await features)
- **Cargo** - Rust package manager
- **OpenSSL** - For SSL certificate generation (installed via dependencies)

### Dependencies (automatically managed by Cargo)
- `tokio` - Async runtime for test execution
- `reqwest` - HTTP client for testing
- `tempfile` - Temporary directory management
- `rcgen` - Self-signed certificate generation
- `serde_json` - JSON validation and parsing

All dependencies are declared in `Cargo.toml` and automatically installed.

## Usage Examples

### Development Workflow
```bash
# Quick development test (specific files)
cargo test --test http_server --test post_requests

# Or run by name pattern
cargo test file_serving post

# Full pre-commit testing
cargo test --all-targets

# Watch mode for TDD
cargo watch -x test

# CI/CD integration
RUST_BACKTRACE=1 cargo test --verbose --all-targets
```

### Debugging Failed Tests
```bash
# Run specific test with backtrace
RUST_BACKTRACE=1 cargo test test_name -- --nocapture

# Show output from passing tests too
cargo test -- --nocapture --show-output

# Run single test file
cargo test --test http_server
```

### Advanced Testing
```bash
# Run tests matching pattern (searches function names)
cargo test json

# Run with custom test threads
cargo test -- --test-threads=1

# Generate coverage report (requires cargo-tarpaulin)
cargo tarpaulin --out Html --output-dir coverage/
```

### Understanding Test Filtering

Cargo provides two ways to filter tests:

**1. By test file** (use `--test` flag):
```bash
cargo test --test http_server    # Runs all tests in tests/http_server.rs
```

**2. By function name** (no flag, just pattern):
```bash
cargo test file_serving           # Runs any test function containing "file_serving"
cargo test basic_file_serving     # Runs exact test function name
```

**Key difference**:
- `--test <file>` runs entire test files
- `<pattern>` searches within test function names across all files

## Adding New Tests

When extending the test framework:

### 1. Individual Test Functions
Add to existing test suite files:

```rust
#[tokio::test]
async fn test_new_feature() {
    let server = TestServer::new().await.expect("Failed to start server");

    // Test implementation
    let response = reqwest::get(server.url())
        .await
        .expect("Request failed");

    response.assert_status(StatusCode::OK);
}
```

### 2. New Test Categories
Create a new test file (e.g., `tests/new_category.rs`):

```rust
mod common;

use common::server::TestServer;
use common::assertions::ResponseAssertions;

#[tokio::test]
async fn test_new_functionality() {
    // Test implementation
}
```

### 3. Shared Utilities
Add to `tests/common/` modules:
- `server.rs` - Server management helpers
- `filesystem.rs` - File creation utilities
- `assertions.rs` - Response validation traits

### 4. Update Documentation
Update this README.md with new test descriptions.

## Testing Best Practices

### Test Design
- **Isolation** - Each test uses unique ports and temp directories
- **Clear assertions** - Use ResponseAssertions trait for validation
- **Error handling** - Test both success and failure paths
- **Resource cleanup** - Leverage RAII with TestServer Drop trait
- **Async/await** - Use `#[tokio::test]` for async tests

### Test Execution
- **Parallel by default** - Tests run concurrently unless `--test-threads=1`
- **Deterministic** - No shared state between tests
- **Fast feedback** - Average test suite completion < 2 seconds
- **CI/CD ready** - Works on all platforms (Linux, macOS, Windows)

### Performance Considerations
- **Port allocation** - Dynamic ports prevent conflicts
- **Temporary files** - Automatic cleanup prevents disk bloat
- **Resource limits** - Tests designed for parallel execution
- **Timeout handling** - Built-in timeouts prevent hanging tests

## Integration with msaada Development

This testing framework integrates with the msaada development workflow:

- **Build verification** - Tests compile and run the binary automatically
- **Feature validation** - All features from main README.md are tested
- **Regression prevention** - Comprehensive coverage catches breaking changes
- **Development feedback** - Fast, focused testing during development
- **Release validation** - Full test suite for release candidates
- **CI/CD pipeline** - Used in GitHub Actions workflows

## Migration from Shell Scripts

This Rust test suite provides **100% functional parity** with the legacy shell script tests while offering significant improvements:

### Coverage Comparison
- **38 shell functions** → **41 Rust tests** (with 50+ sub-tests)
- **4,786 lines** of shell → **2,531 lines** of Rust (47% reduction)
- **Serial execution** → **Parallel execution** (5-10x faster)
- **Platform-specific** → **Cross-platform** (Linux, macOS, Windows)

### Advantages Over Shell Scripts
✅ **Better error reporting** - Full stack traces with RUST_BACKTRACE
✅ **IDE integration** - Run/debug from editor
✅ **Type safety** - Compile-time validation
✅ **Parallel testing** - Isolated test execution
✅ **Better maintainability** - Shared utility modules
✅ **No external dependencies** - Pure Rust with cargo

For migration details, see [todo-rust-integration.md](todo-rust-integration.md).

## Continuous Integration

The test suite is used in GitHub Actions workflows:

- **CI Pipeline** (`.github/workflows/ci.yaml`) - Full test suite on push/PR
- **Quick Tests** (`.github/workflows/test.yaml`) - Fast feedback on PRs
- **Release Gate** (`.github/workflows/release.yaml`) - Pre-release validation

All workflows use `RUST_BACKTRACE=1 cargo test --verbose --all-targets` for comprehensive testing with detailed output.

For more information about msaada features and configuration, see the main project [README.md](../README.md).
