# Msaada

> "Msaada" is Swahili for "service/servant"

A lightweight HTTP server for local web development. Easily serve static files from any directory on your local machine.

## Features

- 🚀 **Fast & Lightweight**: Minimal overhead for quick development
- 🔌 **Easy to Use**: Simple command-line interface
- 🔧 **Flexible**: Serve any directory of your choice
- 📝 **Auto-initialize**: Optionally create basic HTML, CSS, and JavaScript files
- 🔄 **POST Echo**: Accepts POST requests and returns the data as JSON
- 🛡️ **Development Only**: Designed for local development environments

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

```bash
# Basic usage: serve the current directory on port 3000
msaada --port 3000 --dir .

# Serve a specific directory on port 8080
msaada --port 8080 --dir /path/to/website

# Initialize basic web files (HTML, CSS, JS) and serve the directory
msaada --port 3000 --dir /path/to/empty/dir --init

# Get help
msaada --help

# Enable the self-test endpoint to verify POST functionality
msaada --port 3000 --dir . --test
```

## Examples

### Quick static file server

```bash
# Navigate to your project
cd ~/my-web-project

# Serve it on port 3000
msaada --port 3000 --dir .
```

### Creating a new project from scratch

```bash
# Create and navigate to a new directory
mkdir ~/new-project
cd ~/new-project

# Initialize basic web files and serve them
msaada --port 3000 --dir . --init
```

This creates:
- `index.html` - A basic HTML file
- `style.css` - CSS styles for the page
- `main.js` - JavaScript with some interactivity

### Previewing documentation

```bash
# Serve your documentation folder
msaada --port 8080 --dir ./docs
```

### Testing API endpoints

The server can be used as a simple echo server for development. It accepts POST requests and returns the data as JSON:

```bash
# Send a POST request and get the data back as JSON
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

#### Supported POST formats

- **JSON**: `application/json`
- **Form Data**: `application/x-www-form-urlencoded`
- **Multipart Forms**: `multipart/form-data` (with file upload support)
- **Plain Text**: `text/plain`
- **Other Binary Data**: Any other content type

#### Testing POST functionality

Two testing methods are provided:

1. **Comprehensive Test Environment**:

   ```bash
   # Run the comprehensive test suite (starts a server automatically)
   ./tests/run_test.sh
   ```

   This script:
   - Starts a msaada server on port 3099
   - Runs automated tests with curl
   - Provides a browser-based test interface
   - Shows detailed test results

2. **Simple CLI Test**:

   ```bash
   # Run a simple test against a running server
   ./tests/test_post.sh

   # Or specify a custom port
   ./tests/test_post.sh 8080
   ```

3. **Self-Test Endpoint**:

   When running with the `--test` flag, a self-test endpoint is available:

   ```bash
   # Start server with self-test enabled
   msaada --port 3000 --dir . --test

   # Then visit this URL in your browser
   http://localhost:3000/self-test
   ```

## Notes

- This is intended for **development use only** and is not suitable for production environments
- The server binds to `127.0.0.1` (localhost) for security reasons
- All file paths are relative to the specified directory

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Copyright

Copyright © 2022-2025 Vincent Bruijn (vebruijn@gmail.com)