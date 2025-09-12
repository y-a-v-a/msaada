# Msaada

> "Msaada" is Swahili for "service/servant"

A powerful, lightweight HTTP server for local web development. Easily serve static files from any directory with advanced features like HTTPS support, SPA routing, and automatic configuration.

## Features

- 🚀 **Fast & Lightweight**: Built with Rust and Actix-web for optimal performance
- 🔒 **HTTPS/SSL Support**: Serve over HTTPS with PEM or PKCS12 certificates
- ⚙️ **Configuration Files**: Support for serve.json, package.json, and now.json configs
- 🎨 **Colored Logging**: Beautiful terminal output with timestamps and color coding
- 🌐 **CORS Support**: Enable cross-origin requests for API development
- 📦 **Gzip Compression**: Automatic response compression for faster transfers
- 🚦 **Smart Port Management**: Automatic port switching when ports are occupied
- 📋 **Clipboard Integration**: Automatically copy server URL to clipboard
- 🏃 **SPA Support**: Single Page Application routing with --single flag
- ⚡ **ETag/Caching**: Smart caching with ETag and Last-Modified headers
- 🔗 **Symlinks Support**: Follow symbolic links in your file system
- 🛑 **Graceful Shutdown**: Clean server shutdown on SIGINT/SIGTERM
- 📝 **Auto-Initialize**: Create basic HTML, CSS, and JavaScript files
- 🔄 **POST Echo**: Accept POST requests and return data as JSON
- 🛡️ **Development Focused**: Optimized for local development workflows

## Installation

### Using pre-built binaries

Download the latest release from [GitHub Releases](https://github.com/y-a-v-a/msaada/releases) and extract it to a directory in your PATH.

### Building from source

```bash
# Clone the repository
git clone https://github.com/y-a-v-a/msaada.git
cd msaada

# Build in release mode
cargo build --release

# The binary will be in target/release/msaada
```

## Usage

### Basic Usage

```bash
# Serve the current directory on port 3000
msaada --port 3000 --dir .

# Serve a specific directory on port 8080
msaada --port 8080 --dir /path/to/website

# Initialize basic web files and serve
msaada --port 3000 --dir /path/to/empty/dir --init
```

### Advanced Usage

```bash
# Serve with HTTPS
msaada --port 443 --dir . \
  --ssl-cert /path/to/cert.pem \
  --ssl-key /path/to/key.pem

# Single Page Application with CORS
msaada --port 3000 --dir ./dist \
  --single \
  --cors

# Development server with all features
msaada --port 3000 --dir ./src \
  --cors \
  --single \
  --no-clipboard
```

## Command-Line Options

### Basic Options

- `--port, -p <PORT>` - Port number to serve on (required)
- `--dir, -d <DIRECTORY>` - Directory to serve files from (required)
- `--init` - Initialize basic web files (index.html, style.css, main.js)
- `--test` - Enable self-test endpoint at /self-test
- `--config <PATH>` - Specify custom path to serve.json configuration file

### SSL/TLS Options

- `--ssl-cert <PATH>` - Path to SSL/TLS certificate (PEM or PKCS12 format)
- `--ssl-key <PATH>` - Path to SSL/TLS private key (for PEM certificates)
- `--ssl-pass <PATH>` - Path to file containing certificate passphrase

### Web Features

- `--cors` - Enable CORS headers (Access-Control-Allow-Origin: *)
- `--single` - SPA mode: rewrite all not-found requests to index.html
- `--no-compression` - Disable gzip compression
- `--symlinks` - Follow symbolic links instead of showing 404
- `--no-etag` - Use Last-Modified header instead of ETag for caching

### Development Options

- `--no-request-logging` - Disable request logging to console
- `--no-clipboard` - Don't copy server URL to clipboard
- `--no-port-switching` - Don't switch ports when specified port is taken

## Configuration Files

Msaada supports configuration through JSON files with the following precedence:

1. `serve.json` (highest priority)
2. `now.json`
3. `package.json` (under "static" field)

### Example serve.json

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
  "directoryListing": false
}
```

## Examples

### HTTPS Development Server

```bash
# Generate self-signed certificate (for development only)
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes

# Serve with HTTPS
msaada --port 443 --dir ./public \
  --ssl-cert cert.pem \
  --ssl-key key.pem
```

### React/Vue/Angular Development

```bash
# Serve SPA with hot-reload friendly settings
msaada --port 3000 --dir ./build \
  --single \
  --cors \
  --no-etag
```

### API Mock Server

```bash
# Enable CORS and logging for API development
msaada --port 8080 --dir ./mocks \
  --cors \
  --test
```

### Static Documentation Site

```bash
# Serve docs with compression and caching
msaada --port 8080 --dir ./docs
# Compression and ETag are enabled by default
```

## POST Request Handling

The server accepts POST requests and echoes them back as JSON:

### Supported Content Types

- **JSON**: `application/json`
- **Form Data**: `application/x-www-form-urlencoded`
- **Multipart Forms**: `multipart/form-data` (with file uploads)
- **Plain Text**: `text/plain`
- **Binary Data**: Any other content type

### Example POST Request

```bash
curl -X POST \
  -H "Content-Type: application/json" \
  -d '{"name":"John","age":30}' \
  http://localhost:3000/api/test
```

Response:
```json
{
  "path": "/api/test",
  "content_type": "application/json",
  "json_data": {
    "name": "John",
    "age": 30
  }
}
```

## Testing

### Run Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture
```

### Test POST Functionality

```bash
# Comprehensive test suite
./tests/run_test.sh

# Simple POST test
./tests/test_post.sh 3000
```

## Architecture

Msaada is built with a modular architecture:

- **Core Server**: Actix-web based HTTP/HTTPS server
- **File Serving**: Static file serving with actix-files
- **Configuration**: JSON-based configuration system
- **Logging**: Structured, colored logging system
- **Network**: Port management and network utilities
- **TLS**: SSL/TLS certificate handling
- **Middleware**: CORS, compression, headers
- **Utilities**: Clipboard, SPA support, graceful shutdown

## Security Notes

- This server is intended for **development use only**
- By default, binds to localhost for security
- HTTPS support uses standard TLS libraries
- No authentication or authorization built-in
- File serving is restricted to specified directory

## Troubleshooting

### Port Already in Use

The server will automatically try another port unless `--no-port-switching` is used:
```bash
# Let server find available port
msaada --port 3000 --dir .

# Force specific port
msaada --port 3000 --dir . --no-port-switching
```

### Certificate Issues

For PKCS12 certificates:
```bash
msaada --port 443 --dir . \
  --ssl-cert certificate.pfx \
  --ssl-pass passphrase.txt
```

For PEM certificates:
```bash
msaada --port 443 --dir . \
  --ssl-cert cert.pem \
  --ssl-key key.pem
```

### SPA Routing Issues

Enable SPA mode for client-side routing:
```bash
msaada --port 3000 --dir ./dist --single
```

## Contributing

Contributions are welcome! Please read the [CLAUDE.md](CLAUDE.md) file for development guidelines.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Copyright

Copyright © 2022-2025 Vincent Bruijn (vebruijn@gmail.com)