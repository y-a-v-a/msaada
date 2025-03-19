# Msaada

> "Msaada" is Swahili for "service/servant"

A lightweight HTTP server for local web development. Easily serve static files from any directory on your local machine.

## Features

- 🚀 **Fast & Lightweight**: Minimal overhead for quick development
- 🔌 **Easy to Use**: Simple command-line interface
- 🔧 **Flexible**: Serve any directory of your choice
- 📝 **Auto-initialize**: Optionally create basic HTML, CSS, and JavaScript files
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

## Notes

- This is intended for **development use only** and is not suitable for production environments
- The server binds to `127.0.0.1` (localhost) for security reasons
- All file paths are relative to the specified directory

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Copyright

Copyright © 2022-2025 Vincent Bruijn (vebruijn@gmail.com)