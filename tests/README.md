# Msaada Testing

This directory contains scripts for testing the Msaada HTTP server.

## Test Scripts

### test_post.sh

Tests the POST functionality of the server by sending various request types:
- JSON data
- Form-encoded data
- Multipart form with file uploads
- Plain text

Usage:
```bash
# Important: Run from within the tests directory
cd tests
./test_post.sh [PORT]
```

Where `PORT` defaults to 3000 if not provided.

Note: This script must be run from within the tests directory because it references local files for upload testing.

### run_test.sh

Comprehensive testing script that:
1. Starts a server instance on port 3099
2. Tests various content types
3. Provides a browser-based testing interface

Usage:
```bash
./run_test.sh
```

## Test Directory Structure

- `test_server/` - Contains test files for browser-based testing
  - HTML, CSS, and JS files for interactive testing
  - Sample test files for upload testing

## Adding New Tests

When adding new tests:
1. Add individual test scripts to this directory
2. Update run_test.sh if needed for integration testing
3. Document usage in this README.md file

## Testing Best Practices

- Always test both static file serving and POST handling
- Verify custom headers are present in responses
- Test different content types for POST requests
- Test with various file types for uploads