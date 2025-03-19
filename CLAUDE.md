# MSAADA - Rust Project Guidelines

> "Msaada" is Swahili for "service/servant" - A simple HTTP server for local development

## Build & Test Commands
- Build: `cargo build`
- Run: `cargo run -- --port <PORT> --dir <DIRECTORY>`
- Run and initialize files: `cargo run -- --port <PORT> --dir <DIRECTORY> --init`
- Release build: `cargo build --release`
- Package for distribution: `./package.sh`
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
- Can auto-create basic web files:
  - HTML file (src/index_template.html)
  - CSS file (src/style_template.css)
  - JavaScript file (src/main_template.js)

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

## Troubleshooting

- **Port already in use**: If you get "address already in use" errors, try a different port
- **Permission denied**: Ensure you have read/write permissions for the directory you're serving
- **Missing index.html**: Either create one manually or use the `--init` flag to generate one automatically

## Dependencies
- `actix-web` & `actix-files` - Web server framework
- `clap` - Command-line argument parsing
- `log` & `env_logger` - Logging utilities