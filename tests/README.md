# Msaada Testing Framework

This directory contains a comprehensive testing framework for the Msaada HTTP server with dependency-free shell scripts that test all major functionality.

## Quick Start

Run all tests with the comprehensive test runner:
```bash
./run_comprehensive_tests.sh
```

Or run specific test suites:
```bash
./run_comprehensive_tests.sh http post advanced    # Run selected suites
./run_comprehensive_tests.sh --verbose             # Detailed output
./run_comprehensive_tests.sh --help                # Show all options
```

## Test Suites

### Core Test Suites

#### test_http_server.sh
Tests core HTTP server functionality:
- Static file serving and content types
- HTTP methods (GET, HEAD, OPTIONS)
- Response headers and caching
- Error handling (404, 403, etc.)
- Directory indexing and special files

#### test_https_ssl.sh  
Tests HTTPS/SSL integration:
- PEM certificate support (separate cert/key files)
- PKCS12 certificate support (combined .p12/.pfx files)
- Certificate validation and error handling
- Security features and SSL handshakes

#### test_config_files.sh
Tests configuration file system:
- serve.json, now.json, package.json precedence
- Custom configuration paths
- JSON schema validation
- Configuration error handling

#### test_post_enhanced.sh
Enhanced POST request testing:
- JSON POST requests (simple, nested, invalid)
- Form-encoded data (simple, special chars, empty)
- Multipart file uploads (single, multiple, with fields)
- Plain text and binary data handling
- Response format consistency

#### test_advanced_features.sh
Advanced web server features:
- CORS functionality and headers
- Gzip compression (enabled/disabled)
- SPA mode (Single Page Application routing)
- Caching headers (ETag, Last-Modified)
- Symlinks support
- Directory listing
- Configuration-based URL rewrites and headers

#### test_network_ports.sh
Network and port management:
- Port availability checking
- Port conflict handling and auto-switching
- Port boundary cases and validation
- Network interface detection
- Concurrent connections
- IPv6 support (if available)
- Network error handling

### Utility Framework

#### test_utils.sh
Shared utility functions for all test suites:
- Server management (start/stop with cleanup)
- HTTP testing functions (GET, POST, upload)
- Response validation (JSON, headers, content types)
- SSL certificate generation for testing
- Port management and network utilities
- Colored test reporting and progress tracking

## Test Runner

### run_comprehensive_tests.sh
Orchestrates all test suites with advanced options:

**Basic Usage:**
```bash
./run_comprehensive_tests.sh                    # Run all suites
./run_comprehensive_tests.sh --list-suites     # List available suites
./run_comprehensive_tests.sh --dry-run         # Show execution plan
```

**Selective Testing:**
```bash
./run_comprehensive_tests.sh http https        # Run specific suites
./run_comprehensive_tests.sh --selective http,post,advanced
```

**Execution Modes:**
```bash
./run_comprehensive_tests.sh --verbose         # Show detailed output
./run_comprehensive_tests.sh --parallel        # Parallel execution
./run_comprehensive_tests.sh --quick           # Skip slower tests
./run_comprehensive_tests.sh --stop-on-failure # Stop on first failure
```

**Suite Keywords:**
- `http` - Core HTTP server functionality  
- `https` - HTTPS/SSL integration tests
- `config` - Configuration file tests
- `post` - POST request echo tests
- `advanced` - Advanced features (CORS, SPA, etc.)
- `network` - Network and port management tests

## Test Directory Structure

```
tests/
├── README.md                      # This documentation
├── run_comprehensive_tests.sh     # Main test runner
├── test_utils.sh                  # Shared utilities
├── test_http_server.sh           # Core HTTP tests
├── test_https_ssl.sh             # HTTPS/SSL tests  
├── test_config_files.sh          # Configuration tests
├── test_post_enhanced.sh         # Enhanced POST tests
├── test_advanced_features.sh     # Advanced feature tests
├── test_network_ports.sh         # Network/port tests
└── test_server/                  # Test files for manual testing
    ├── index.html
    ├── style.css
    └── various test files...
```

## Test Features

### Comprehensive Coverage
- **600+ individual tests** across all msaada features
- **Zero external dependencies** - uses only standard Unix tools
- **Professional reporting** with colored output and statistics
- **Parallel execution** support for faster testing
- **Selective testing** for focused development
- **Error isolation** with detailed failure reporting

### Robust Testing
- **Automatic server management** with proper cleanup
- **Port conflict resolution** and availability checking
- **SSL certificate generation** for HTTPS testing
- **Concurrent connection testing** for reliability
- **Response validation** for correctness
- **Graceful error handling** with informative messages

### Developer Friendly
- **Flexible execution options** for different workflows
- **Detailed progress tracking** with real-time updates
- **Comprehensive documentation** and help systems
- **Easy integration** with CI/CD pipelines
- **Clear failure diagnostics** for quick debugging

## Requirements

### System Requirements
- **Bash 4.0+** (for associative arrays and advanced features)
- **Standard Unix tools**: curl, openssl, jq, grep, sed, awk
- **Network utilities**: netstat or ss (for port checking)
- **Msaada binary**: Built and available in ../target/release/

### Optional Dependencies
- **OpenSSL**: For SSL certificate generation (HTTPS tests)
- **jq**: For JSON validation and parsing (POST tests)

## Usage Examples

### Development Workflow
```bash
# Quick development test
./run_comprehensive_tests.sh http post --verbose

# Full pre-commit testing
./run_comprehensive_tests.sh --stop-on-failure

# CI/CD integration
./run_comprehensive_tests.sh --quick 2>&1 | tee test-results.log
```

### Debugging Failed Tests
```bash
# Run only failed suite with verbose output
./run_comprehensive_tests.sh https --verbose

# Check test logs (created in .tmp directory)
ls tests/.tmp/
cat tests/.tmp/server_*.log
```

### Adding New Tests

When extending the test framework:

1. **Individual test functions** - Add to existing test suites
2. **New test categories** - Create new test suite script
3. **Utility functions** - Add to test_utils.sh
4. **Update documentation** - Update this README.md

Example test function structure:
```bash
test_new_feature() {
    print_test_start "New Feature Testing"
    
    local tests_passed=0
    
    # Test implementation
    if [[ condition ]]; then
        print_success "Test description"
        tests_passed=$((tests_passed + 1))
    else
        print_failure "Test failed: reason"
    fi
    
    # Return results
    if [[ $tests_passed -eq expected_count ]]; then
        return 0
    else
        return 1
    fi
}
```

## Testing Best Practices

### Test Design
- **Test isolation** - Each test should be independent
- **Clear assertions** - Verify specific expected behaviors  
- **Error handling** - Test both success and failure cases
- **Resource cleanup** - Always clean up test artifacts
- **Consistent reporting** - Use utility functions for output

### Test Execution
- **Run full suite** before commits to main branch
- **Test environment isolation** using temporary directories
- **Port management** to avoid conflicts during parallel testing
- **SSL certificate rotation** for security testing
- **Comprehensive logging** for debugging test failures

### Performance Considerations
- **Parallel execution** for faster feedback in CI/CD
- **Selective testing** during active development
- **Resource monitoring** to avoid system overload
- **Test timeout handling** for reliability
- **Efficient cleanup** to minimize resource usage

## Integration with msaada Development

This testing framework integrates with the msaada development workflow:

- **Build verification**: Automatically builds msaada binary if needed
- **Feature validation**: Tests all features documented in README.md
- **Regression prevention**: Comprehensive coverage prevents regressions
- **Development feedback**: Fast, focused testing during development
- **Release validation**: Full test suite for release candidates

For more information about msaada features and configuration, see the main project README.md.