# MSAADA - Rust Project Guidelines

> "Msaada" is Swahili for "service/servant" - A simple HTTP server for local development

## Build & Test Commands
- Build: `cargo build`
- Run: `cargo run -- --port <PORT> --dir <DIRECTORY>`
- Run and initialize files: `cargo run -- --port <PORT> --dir <DIRECTORY> --init`
- Run with self-test: `cargo run -- --port <PORT> --dir <DIRECTORY> --test`
- Release build: `cargo build --release`
- Package for distribution: `./package.sh`
- Comprehensive testing: `./tests/run_test.sh`
- Simple POST test: `./tests/test_post.sh [PORT]`
- Test: `cargo test`
- Test single test: `cargo test <TEST_NAME>`
- Check only: `cargo check`
- Format code: `cargo fmt`
- Lint: `cargo clippy`
- Update dependencies: `cargo update`

## Code Style Guidelines
- **Imports**: Group std imports first, then external crates, then local modules
- **Formatting**: Use rustfmt (run `cargo fmt` before commits)
- **Error Handling**: Use Result/Option types with pattern matching; avoid unwrap() in production code
- **Naming Conventions**:
  - snake_case for functions, variables, modules
  - CamelCase for types, traits, enums
  - SCREAMING_CASE for constants
- **Documentation**: Document public APIs with /// comments
- **Type Safety**: Use Rust's strong type system; avoid `as` casts when possible
- **Logging**: Use the `log` crate macros (info!, error!, etc.) for logging

## Project Features
- Simple HTTP server for local development
- Serves static files from a specified directory
- Command-line interface with the following arguments:
  - `--port/-p`: Port number to serve on (required)
  - `--dir/-d`: Directory to serve files from (required)
  - `--init`: Initialize basic web files in the directory (optional)
  - `--test`: Enable the self-test endpoint (optional)
- Can auto-create basic web files:
  - HTML file (src/index_template.html)
  - CSS file (src/style_template.css)
  - JavaScript file (src/main_template.js)
- Handles POST requests and returns data as JSON:
  - Multipart form data (including file uploads)
  - JSON data
  - URL-encoded form data
  - Plain text
  - Binary data
- Custom response headers:
  - `X-Server`: Server name and version (msaada/0.1.0)
  - `X-Powered-By`: Server name (msaada)
  - `X-Version`: Server version (0.1.0)

## Usage Examples

```bash
# Serve the current directory on port 3000 
msaada --port 3000 --dir .

# Serve a specific project folder on port 8080
msaada --port 8080 --dir /path/to/project

# Initialize basic web files in a directory and serve it
msaada --port 3000 --dir /path/to/empty/folder --init
```

## Project Structure
- `src/main.rs` - Main application code
- `src/index_template.html` - Template for generated HTML file
- `src/style_template.css` - Template for generated CSS file
- `src/main_template.js` - Template for generated JavaScript file
- `package.sh` - Script to build and package for distribution
- `tests/` - Test infrastructure directory
  - `run_test.sh` - Comprehensive testing script
  - `test_post.sh` - Simple CLI test script
  - `test_server/` - Test files for browser-based testing

## Troubleshooting

- **Port already in use**: If you get "address already in use" errors, try a different port
- **Permission denied**: Ensure you have read/write permissions for the directory you're serving
- **Missing index.html**: Either create one manually or use the `--init` flag to generate one automatically
- **405 Method Not Allowed**: This error indicates a routing problem
  - Check if the server was started with the latest version
  - Ensure your POST requests are being made to the correct endpoint
  - Run the test script to verify POST functionality: `./tests/run_test.sh`
- **404 Not Found**: Make sure you're using the correct port in your requests
  - The server only listens on the specified port (e.g., 3000)
  - Make sure files exist in the directory being served

## Dependencies
- `actix-web` & `actix-files` - Web server framework
- `actix-multipart` - Multipart form handling
- `clap` - Command-line argument parsing
- `log` & `env_logger` - Logging utilities
- `serde` & `serde_json` - JSON serialization/deserialization
- `futures-util` - Async utilities
- `mime` - MIME type handling
- `urlencoding` - URL encoding/decoding

## POST Request API

The server can handle POST requests to any endpoint and will return the data as JSON:

### Example POST request:

```bash
# Using curl to send a multipart form with a file
curl -X POST \
  -F "name=John Smith" \
  -F "message=Hello from curl" \
  -F "file=@/path/to/file.txt" \
  http://localhost:3000/api/test
```

### Response Format:

```json
{
  "path": "/api/test",
  "content_type": "multipart/form-data; boundary=...",
  "form_data": {
    "name": "John Smith",
    "message": "Hello from curl"
  },
  "files": [
    {
      "field_name": "file",
      "filename": "file.txt"
    }
  ]
}
```

Note: The server does not actually save uploaded files - it only returns their filenames in the JSON response.

### Testing POST Functionality

#### Automated Testing

```bash
# Run the comprehensive test suite (starts server on port 3099)
./tests/run_test.sh

# The script will:
# 1. Start the server on a dedicated port (3099)
# 2. Run tests for various content types
# 3. Keep the server running for manual testing
# 4. Provide a browser-based test interface
```

#### Browser-Based Testing

After running `./tests/run_test.sh`, open your browser to:
```
http://localhost:3099
```

This provides an interactive interface for testing:
- Form submission with file upload
- JSON POST requests
- Plain text POST requests

#### Supported Content Types

1. **JSON**: `application/json`
2. **Form Data**: `application/x-www-form-urlencoded`
3. **Multipart Form**: `multipart/form-data` (with file uploads)
4. **Plain Text**: `text/plain`
5. **Other**: Any other content type (treated as binary)

## Architecture Details

- **Request Handling**: 
  - Uses Actix's route handlers with specific routing order (wildcard POST handler registered after specific routes)
  - Static file serving via actix-files integration
  - Middleware for custom header injection
  
- **POST Processing Pipeline**:
  - Content-type based branching in `handle_post()` function
  - Specialized handling for different content types
  - No persistent storage of uploaded files
  - Automatic serialization of response data to JSON

- **Custom Header Implementation**:
  - Uses Actix's DefaultHeaders middleware
  - Headers generated from Cargo environment variables (package name/version)
  - Consistent application across all response types

## Future Development

- **Testing Improvements**:
  - Add unit tests using Rust's built-in testing framework
  - Create tests for POST handler with different content types
  - Test template file generation logic
  - Test command line argument parsing
  - Test middleware behavior

- **Feature Enhancements**:
  - Security review for file upload handling
  - Support for more content types
  - Configuration file support
  - More sophisticated routing capabilities
  - Optional authentication for certain operations
  - HTTPS support