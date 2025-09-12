# MSAADA - Rust Project Guidelines

> "Msaada" is Swahili for "service/servant" - A powerful HTTP server for local development

## Build & Test Commands

- Build: `cargo build`
- Run: `cargo run -- --port <PORT> --dir <DIRECTORY>`
- Run with HTTPS: `cargo run -- --port <PORT> --dir <DIRECTORY> --ssl-cert <CERT> --ssl-key <KEY>`
- Run with config: `cargo run -- --port <PORT> --dir <DIRECTORY> --config serve.json`
- Run SPA mode: `cargo run -- --port <PORT> --dir <DIRECTORY> --single`
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
- **Logging**: Use the structured logger module with colored output

## Module Architecture

### Core Modules

- **`main.rs`** - Application entry point, CLI parsing, server initialization
- **`clipboard.rs`** - Clipboard integration for copying server URLs
- **`config.rs`** - Configuration file parsing (serve.json, package.json, now.json)
- **`logger.rs`** - Structured logging with colors and timestamps
- **`network.rs`** - Port availability checking and network utilities
- **`shutdown.rs`** - Graceful shutdown handling for SIGINT/SIGTERM
- **`spa.rs`** - Single Page Application support utilities
- **`tls.rs`** - SSL/TLS certificate loading and configuration

### Template Files

- **`index_template.html`** - Template for generated HTML file
- **`style_template.css`** - Template for generated CSS file
- **`main_template.js`** - Template for generated JavaScript file

## Complete Feature Set

### Core Features
- HTTP/HTTPS server with Actix-web
- Static file serving from specified directory
- POST request handling with JSON echo response
- Template file initialization (--init flag)
- Self-test endpoint for POST validation

### Advanced Features

#### SSL/TLS Support
- PEM format certificates (separate cert and key files)
- PKCS12/PFX format certificates (combined file)
- Passphrase support for encrypted certificates
- Automatic format detection based on file extension

#### Configuration System
- JSON configuration file support
- Multiple config file formats: serve.json, now.json, package.json
- Configuration precedence rules
- Schema validation with helpful error messages
- Custom config path via --config flag

#### Logging & Monitoring
- Colored terminal output with log levels (INFO, WARN, ERROR)
- Timestamp formatting for all messages
- Request/response logging with timing
- Configurable via --no-request-logging flag

#### Network Features
- CORS support with configurable headers
- Gzip compression (enabled by default)
- Port availability checking
- Automatic port switching when occupied
- Network interface detection for external IP

#### Web Development Features
- Single Page Application (SPA) mode
- ETag and Last-Modified caching headers
- Symlinks support
- Clean URLs and trailing slash handling
- URL rewrites and redirects (via config)

#### User Experience
- Clipboard integration (auto-copy server URL)
- Colored server startup messages
- Graceful shutdown on signals
- Smart error messages and hints

## CLI Arguments Reference

### Required Arguments
- `--port, -p <PORT>` - Port number to serve on
- `--dir, -d <DIRECTORY>` - Directory to serve files from

### Optional Arguments

#### Basic Options
- `--init` - Initialize basic web files (index.html, style.css, main.js)
- `--test` - Enable self-test endpoint at /self-test
- `--config <PATH>` - Specify custom path to configuration file

#### SSL/TLS Options
- `--ssl-cert <PATH>` - Path to SSL/TLS certificate (PEM or PKCS12)
- `--ssl-key <PATH>` - Path to private key (for PEM certificates)
- `--ssl-pass <PATH>` - Path to passphrase file

#### Web Features
- `--cors` - Enable CORS headers
- `--single` - SPA mode (rewrite not-found to index.html)
- `--no-compression` - Disable gzip compression
- `--symlinks` - Follow symbolic links
- `--no-etag` - Use Last-Modified instead of ETag

#### Development Options
- `--no-request-logging` - Disable request logging
- `--no-clipboard` - Don't copy URL to clipboard
- `--no-port-switching` - Don't auto-switch ports

## Dependencies

### Web Framework
- `actix-web` - Core web server framework
- `actix-files` - Static file serving
- `actix-multipart` - Multipart form handling
- `actix-cors` - CORS middleware
- `awc` - Actix Web Client for self-testing

### TLS/Security
- `rustls` - TLS implementation
- `rustls-pemfile` - PEM file parsing
- `tokio-rustls` - Async TLS
- `p12` - PKCS12 certificate support
- `actix-web-httpauth` - HTTP authentication

### Configuration & Parsing
- `clap` - Command-line argument parsing
- `serde` & `serde_json` - JSON serialization
- `jsonschema` - JSON schema validation
- `mime` - MIME type handling
- `urlencoding` - URL encoding/decoding

### Utilities
- `tokio` - Async runtime
- `futures-util` - Async utilities
- `bytes` - Byte buffer utilities
- `chrono` - Date and time handling
- `colored` - Terminal colors
- `clipboard` - Clipboard access
- `port_check` - Port availability checking
- `local-ip-address` - Network interface detection
- `flate2` - Compression support
- `signal-hook` & `signal-hook-tokio` - Signal handling
- `atty` - Terminal detection

### Development
- `env_logger` & `log` - Logging framework
- `tempfile` - Temporary file handling (dev dependency)

## Configuration File Format

### serve.json Example

```json
{
  "public": "dist",
  "cleanUrls": true,
  "trailingSlash": false,
  "rewrites": [
    { "source": "/api/(.*)", "destination": "/api/index.html" }
  ],
  "headers": [
    {
      "source": "**/*.@(jpg|jpeg|gif|png)",
      "headers": [
        { "key": "Cache-Control", "value": "max-age=7200" }
      ]
    }
  ],
  "directoryListing": false,
  "etag": true,
  "symlinks": false,
  "compress": true
}
```

### Configuration Precedence

1. Command-line arguments (highest priority)
2. serve.json (if exists)
3. now.json (if exists)
4. package.json "static" field (if exists)
5. Default values (lowest priority)

## POST Request API

### Endpoint
Any path accepts POST requests and returns data as JSON.

### Supported Content Types
1. **JSON**: `application/json`
2. **Form Data**: `application/x-www-form-urlencoded`
3. **Multipart Form**: `multipart/form-data` (with file uploads)
4. **Plain Text**: `text/plain`
5. **Binary**: Any other content type

### Response Format

```json
{
  "path": "/api/endpoint",
  "content_type": "application/json",
  "json_data": { ... },
  "form_data": { ... },
  "text_data": "...",
  "files": [
    {
      "field_name": "upload",
      "filename": "document.pdf"
    }
  ]
}
```

## Testing

### Unit Tests
Each module includes unit tests. Run with:
```bash
cargo test                 # All tests
cargo test config::        # Module tests
cargo test -- --nocapture  # With output
```

### Integration Tests
```bash
./tests/run_test.sh        # Full test suite
./tests/test_post.sh 3000  # POST endpoint test
```

### Manual Testing
```bash
# Start with test endpoint
cargo run -- --port 3000 --dir . --test

# Visit in browser
http://localhost:3000/self-test
```

## Troubleshooting

### Common Issues

1. **Port Already in Use**
   - Server auto-switches to available port
   - Use `--no-port-switching` to force specific port

2. **Certificate Errors**
   - Verify certificate format (PEM vs PKCS12)
   - Check file paths are correct
   - Ensure private key matches certificate

3. **Configuration Not Loading**
   - Check JSON syntax is valid
   - Verify file is named correctly (serve.json)
   - Use `--config` to specify custom path

4. **SPA Routing Issues**
   - Enable `--single` flag for client-side routing
   - Configure rewrites in serve.json if needed

5. **CORS Errors**
   - Add `--cors` flag to enable CORS headers
   - Configure specific origins in serve.json

## Architecture Details

### Request Flow
1. Request received by Actix-web server
2. Middleware processing (CORS, compression, headers)
3. Route matching (POST handler or static files)
4. Response generation with appropriate headers
5. Logging and metrics collection

### Module Responsibilities

- **Core Server** (`main.rs`): Initialization, routing, middleware setup
- **Configuration** (`config.rs`): File parsing, validation, precedence
- **Logging** (`logger.rs`): Structured output, formatting, colors
- **Network** (`network.rs`): Port management, IP detection
- **TLS** (`tls.rs`): Certificate loading, format detection
- **Shutdown** (`shutdown.rs`): Signal handling, cleanup
- **SPA** (`spa.rs`): Route rewriting, fallback handling
- **Clipboard** (`clipboard.rs`): Cross-platform clipboard access

### Error Handling Strategy

- Use custom error types for each module
- Propagate errors with `?` operator
- Provide helpful error messages to users
- Log errors with appropriate severity levels
- Graceful fallbacks where possible

## Future Development

### Planned Features
- Directory listing UI with customizable templates
- WebSocket support for live reload
- Request/response middleware plugins
- Authentication and authorization options
- Metrics and monitoring endpoints
- HTTP/2 and HTTP/3 support

### Performance Optimizations
- Caching layer for frequently accessed files
- Connection pooling for keep-alive
- Optimized static file serving
- Memory-mapped file support

### Security Enhancements
- Rate limiting middleware
- Security headers by default
- CSP (Content Security Policy) support
- Request validation and sanitization

## Contributing Guidelines

1. **Code Quality**
   - Run `cargo fmt` before committing
   - Run `cargo clippy` and fix warnings
   - Add tests for new functionality
   - Update documentation

2. **Pull Requests**
   - Create feature branch from main
   - Write descriptive commit messages
   - Include tests and documentation
   - Ensure CI passes

3. **Testing Requirements**
   - Unit tests for new modules
   - Integration tests for features
   - Manual testing checklist
   - Performance benchmarks for critical paths

## License

MIT License - See LICENSE file for details

## Copyright

Copyright © 2022-2025 Vincent Bruijn (vebruijn@gmail.com)