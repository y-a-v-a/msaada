//! File system testing utilities
//!
//! This module provides utilities for creating test files, directories,
//! and managing file system operations during testing.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

/// File system testing helpers
pub struct FileSystemHelper;

impl FileSystemHelper {
    /// Create a test HTML file
    pub fn create_html_file(
        dir: &Path,
        filename: &str,
        title: &str,
        body_content: &str,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let content = format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{}</title>
</head>
<body>
    {}
</body>
</html>"#,
            title, body_content
        );

        let filepath = dir.join(filename);
        std::fs::write(&filepath, content)?;
        Ok(filepath)
    }

    /// Create a test CSS file
    pub fn create_css_file(
        dir: &Path,
        filename: &str,
        styles: &str,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let filepath = dir.join(filename);
        std::fs::write(&filepath, styles)?;
        Ok(filepath)
    }

    /// Create a test JavaScript file
    pub fn create_js_file(
        dir: &Path,
        filename: &str,
        code: &str,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let filepath = dir.join(filename);
        std::fs::write(&filepath, code)?;
        Ok(filepath)
    }

    /// Create a test JSON file
    pub fn create_json_file(
        dir: &Path,
        filename: &str,
        data: &Value,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(data)?;
        let filepath = dir.join(filename);
        std::fs::write(&filepath, content)?;
        Ok(filepath)
    }

    /// Create a test configuration file (serve.json, package.json, etc.)
    pub fn create_config_file(
        dir: &Path,
        config_type: ConfigType,
        config: &Value,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let filename = match config_type {
            ConfigType::Serve => "serve.json",
            ConfigType::Now => "now.json",
            ConfigType::Package => "package.json",
        };

        Self::create_json_file(dir, filename, config)
    }

    /// Create a directory structure with test files
    pub fn create_test_structure(
        base_dir: &Path,
        structure: &TestStructure,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for (path, content) in &structure.files {
            let filepath = base_dir.join(path);

            // Create parent directories if needed
            if let Some(parent) = filepath.parent() {
                std::fs::create_dir_all(parent)?;
            }

            match content {
                FileContent::Html { title, body } => {
                    Self::create_html_file(
                        filepath.parent().unwrap(),
                        filepath.file_name().unwrap().to_str().unwrap(),
                        title,
                        body,
                    )?;
                }
                FileContent::Text(text) => {
                    std::fs::write(&filepath, text)?;
                }
                FileContent::Json(json) => {
                    let content = serde_json::to_string_pretty(json)?;
                    std::fs::write(&filepath, content)?;
                }
                FileContent::Binary(data) => {
                    std::fs::write(&filepath, data)?;
                }
            }
        }

        Ok(())
    }

    /// Validate file content matches expected content
    pub fn validate_file_content(
        filepath: &Path,
        expected: &str,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let actual = std::fs::read_to_string(filepath)?;
        Ok(actual.trim() == expected.trim())
    }

    /// Create a serve.json configuration file
    pub fn create_serve_json(
        dir: &Path,
        public_dir: &str,
        options: Option<ServeJsonOptions>,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let mut config = serde_json::json!({
            "public": public_dir
        });

        if let Some(opts) = options {
            if let Some(clean_urls) = opts.clean_urls {
                config["cleanUrls"] = serde_json::Value::Bool(clean_urls);
            }
            if let Some(trailing_slash) = opts.trailing_slash {
                config["trailingSlash"] = serde_json::Value::Bool(trailing_slash);
            }
            if let Some(etag) = opts.etag {
                config["etag"] = serde_json::Value::Bool(etag);
            }
            if let Some(directory_listing) = opts.directory_listing {
                config["directoryListing"] = serde_json::Value::Bool(directory_listing);
            }
            if let Some(symlinks) = opts.symlinks {
                config["symlinks"] = serde_json::Value::Bool(symlinks);
            }
            if !opts.rewrites.is_empty() {
                config["rewrites"] = serde_json::Value::Array(opts.rewrites);
            }
            if !opts.redirects.is_empty() {
                config["redirects"] = serde_json::Value::Array(opts.redirects);
            }
            if !opts.headers.is_empty() {
                config["headers"] = serde_json::Value::Array(opts.headers);
            }
        }

        Self::create_json_file(dir, "serve.json", &config)
    }

    /// Create a now.json configuration file (legacy format)
    pub fn create_now_json(
        dir: &Path,
        public_dir: &str,
        options: Option<NowJsonOptions>,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let mut static_config = serde_json::json!({
            "public": public_dir
        });

        if let Some(opts) = options {
            if let Some(clean_urls) = opts.clean_urls {
                static_config["cleanUrls"] = serde_json::Value::Bool(clean_urls);
            }
            if let Some(trailing_slash) = opts.trailing_slash {
                static_config["trailingSlash"] = serde_json::Value::Bool(trailing_slash);
            }
            if let Some(render_single) = opts.render_single {
                static_config["renderSingle"] = serde_json::Value::Bool(render_single);
            }
            if let Some(etag) = opts.etag {
                static_config["etag"] = serde_json::Value::Bool(etag);
            }
            if let Some(directory_listing) = opts.directory_listing {
                static_config["directoryListing"] = serde_json::Value::Bool(directory_listing);
            }
            if let Some(symlinks) = opts.symlinks {
                static_config["symlinks"] = serde_json::Value::Bool(symlinks);
            }
        }

        let config = serde_json::json!({
            "now": {
                "static": static_config
            }
        });

        Self::create_json_file(dir, "now.json", &config)
    }

    /// Create advanced features test files
    pub fn setup_advanced_test_files(
        dir: &Path,
    ) -> Result<AdvancedTestFiles, Box<dyn std::error::Error>> {
        // Create test directory structure
        std::fs::create_dir_all(dir)?;

        let spa_dir = dir.join("spa");
        let subdirectory = dir.join("subdirectory");
        let symlink_target = dir.join("symlink_target");

        std::fs::create_dir_all(&spa_dir)?;
        std::fs::create_dir_all(&subdirectory)?;
        std::fs::create_dir_all(&symlink_target)?;

        // Create basic index page
        let index_html = Self::create_html_file(
            dir,
            "index.html",
            "Advanced Features Test",
            "<h1>Advanced Features Test Server</h1>",
        )?;

        // Create SPA files
        Self::create_html_file(
            &spa_dir,
            "index.html",
            "SPA Root",
            "<h1>SPA Application</h1><div id='app'>Loading...</div>",
        )?;
        Self::create_js_file(&spa_dir, "app.js", "console.log('SPA app loaded');")?;
        Self::create_css_file(&spa_dir, "style.css", "body { font-family: Arial; }")?;

        // Create subdirectory content
        std::fs::write(
            subdirectory.join("content.txt"),
            "This is content in a subdirectory",
        )?;
        Self::create_html_file(
            &subdirectory,
            "page.html",
            "Subdirectory Page",
            "<h2>Subdirectory Content</h2>",
        )?;

        // Create symlink target file
        std::fs::write(
            symlink_target.join("target.txt"),
            "This is the target file for symlink tests",
        )?;

        // Create serve.json configuration
        let serve_json = dir.join("serve.json");
        let config = json!({
            "cleanUrls": true,
            "trailingSlash": false,
            "rewrites": [
                { "source": "/api/(.*)", "destination": "/api.html" },
                { "source": "/old-path", "destination": "/new-path" }
            ],
            "headers": [
                {
                    "source": "**/*.css",
                    "headers": [
                        { "key": "Cache-Control", "value": "max-age=3600" }
                    ]
                }
            ],
            "directoryListing": true,
            "etag": true,
            "compress": true
        });
        std::fs::write(&serve_json, serde_json::to_string_pretty(&config)?)?;

        // Create API mock and redirect target
        Self::create_html_file(
            dir,
            "api.html",
            "API Mock",
            "<h1>API Response</h1><p>This simulates an API endpoint</p>",
        )?;
        Self::create_html_file(
            dir,
            "new-path.html",
            "New Path",
            "<h1>Redirected Content</h1>",
        )?;

        Ok(AdvancedTestFiles {
            index_html,
            spa_dir,
            subdirectory,
            symlink_target,
            serve_json,
        })
    }

    /// Create SSL/HTTPS test files
    pub fn setup_ssl_test_files(
        dir: &Path,
    ) -> Result<SslTestFiles, Box<dyn std::error::Error>> {
        // Create test directory
        std::fs::create_dir_all(dir)?;

        // Create index.html with "Secure Connection" content
        let index_html = Self::create_html_file(
            dir,
            "index.html",
            "HTTPS Test",
            "<h1>Secure Connection</h1><p>This page is served over HTTPS.</p>",
        )?;

        // Create api.json
        let api_json = dir.join("api.json");
        let api_data = serde_json::json!({
            "status": "secure",
            "protocol": "https",
            "message": "SSL working"
        });
        std::fs::write(&api_json, serde_json::to_string_pretty(&api_data)?)?;

        Ok(SslTestFiles {
            index_html,
            api_json,
        })
    }

    /// Create POST test files for upload testing
    pub fn setup_post_test_files(
        dir: &Path,
    ) -> Result<PostTestFiles, Box<dyn std::error::Error>> {
        // Create test directory
        std::fs::create_dir_all(dir)?;

        // Create basic index page
        let index_file = Self::create_html_file(
            dir,
            "index.html",
            "POST Test Server",
            "<h1>POST Testing Server</h1><p>Use this server to test POST requests.</p>",
        )?;

        // Create sample text file
        let sample_txt = dir.join("sample.txt");
        std::fs::write(
            &sample_txt,
            "This is a sample text file for upload testing.",
        )?;

        // Create sample JSON file
        let sample_json = dir.join("sample.json");
        std::fs::write(&sample_json, r#"{"test": "json", "upload": true}"#)?;

        // Create sample PNG file (minimal valid 1x1 PNG)
        let sample_png = dir.join("sample.png");
        let png_data: Vec<u8> = vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
            0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR chunk
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // 1x1 dimensions
            0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, // IHDR data
            0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, // IDAT chunk
            0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, // IDAT data
            0x0D, 0x0A, 0x2D, 0xDB, // IDAT checksum
            0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, // IEND chunk
            0xAE, 0x42, 0x60, 0x82, // IEND checksum
        ];
        std::fs::write(&sample_png, png_data)?;

        // Create large binary file (100KB)
        let large_file = dir.join("large_file.bin");
        let large_data = vec![0u8; 102400]; // 100KB of zeros
        std::fs::write(&large_file, large_data)?;

        Ok(PostTestFiles {
            index_html: index_file,
            sample_txt,
            sample_json,
            sample_png,
            large_file,
        })
    }

    /// Create a package.json configuration file with static section
    pub fn create_package_json(
        dir: &Path,
        public_dir: &str,
        options: Option<PackageJsonOptions>,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let mut static_config = serde_json::json!({
            "public": public_dir
        });

        let (name, version) = if let Some(ref opts) = options {
            if let Some(clean_urls) = opts.clean_urls {
                static_config["cleanUrls"] = serde_json::Value::Bool(clean_urls);
            }
            if let Some(render_single) = opts.render_single {
                static_config["renderSingle"] = serde_json::Value::Bool(render_single);
            }
            if let Some(etag) = opts.etag {
                static_config["etag"] = serde_json::Value::Bool(etag);
            }
            (
                opts.name.as_deref().unwrap_or("test-app"),
                opts.version.as_deref().unwrap_or("1.0.0")
            )
        } else {
            ("test-app", "1.0.0")
        };

        let config = serde_json::json!({
            "name": name,
            "version": version,
            "description": "Test application",
            "main": "index.js",
            "scripts": {
                "start": "node index.js"
            },
            "static": static_config,
            "dependencies": {
                "express": "^4.18.0"
            }
        });

        Self::create_json_file(dir, "package.json", &config)
    }

    /// Create a multi-directory configuration test environment
    pub fn setup_config_test_environment(
        base_dir: &Path,
    ) -> Result<ConfigTestEnvironment, Box<dyn std::error::Error>> {
        // Create subdirectories
        let public_dir = base_dir.join("public");
        let dist_dir = base_dir.join("dist");
        let build_dir = base_dir.join("build");
        let static_dir = base_dir.join("static");

        std::fs::create_dir_all(&public_dir)?;
        std::fs::create_dir_all(&dist_dir)?;
        std::fs::create_dir_all(&build_dir)?;
        std::fs::create_dir_all(&static_dir)?;

        // Create API directories
        std::fs::create_dir_all(public_dir.join("api"))?;
        std::fs::create_dir_all(dist_dir.join("api"))?;
        std::fs::create_dir_all(base_dir.join("api"))?;

        // Create index files for each directory
        Self::create_html_file(
            base_dir,
            "index.html",
            "Base Index",
            "<h1>Base Directory</h1>",
        )?;
        Self::create_html_file(
            &public_dir,
            "index.html",
            "Public Index",
            "<h1>Public Directory</h1>",
        )?;
        Self::create_html_file(
            &dist_dir,
            "index.html",
            "Dist Index",
            "<h1>Dist Directory</h1>",
        )?;
        Self::create_html_file(
            &build_dir,
            "index.html",
            "Build Index",
            "<h1>Build Directory</h1>",
        )?;
        Self::create_html_file(
            &static_dir,
            "index.html",
            "Static Index",
            "<h1>Static Directory</h1>",
        )?;

        // Create API test files
        let base_api_data = serde_json::json!({"source": "base", "config": "none"});
        let public_api_data = serde_json::json!({"source": "public", "config": "serve.json"});
        let dist_api_data = serde_json::json!({"source": "dist", "config": "package.json"});

        Self::create_json_file(&base_dir.join("api"), "test.json", &base_api_data)?;
        Self::create_json_file(&public_dir.join("api"), "test.json", &public_api_data)?;
        Self::create_json_file(&dist_dir.join("api"), "test.json", &dist_api_data)?;

        Ok(ConfigTestEnvironment {
            base_dir: base_dir.to_path_buf(),
            public_dir,
            dist_dir,
            build_dir,
            static_dir,
        })
    }
}

/// Configuration file types
#[derive(Debug, Clone)]
pub enum ConfigType {
    Serve,
    Now,
    Package,
}

/// Test file content types
#[derive(Debug, Clone)]
pub enum FileContent {
    Html { title: String, body: String },
    Text(String),
    Json(Value),
    Binary(Vec<u8>),
}

/// Test directory structure definition
#[derive(Debug, Clone)]
pub struct TestStructure {
    pub files: HashMap<PathBuf, FileContent>,
}

impl TestStructure {
    pub fn new() -> Self {
        TestStructure {
            files: HashMap::new(),
        }
    }

    pub fn add_html_file(mut self, path: &str, title: &str, body: &str) -> Self {
        self.files.insert(
            PathBuf::from(path),
            FileContent::Html {
                title: title.to_string(),
                body: body.to_string(),
            },
        );
        self
    }

    pub fn add_text_file(mut self, path: &str, content: &str) -> Self {
        self.files
            .insert(PathBuf::from(path), FileContent::Text(content.to_string()));
        self
    }

    pub fn add_json_file(mut self, path: &str, data: Value) -> Self {
        self.files
            .insert(PathBuf::from(path), FileContent::Json(data));
        self
    }

    pub fn add_binary_file(mut self, path: &str, data: Vec<u8>) -> Self {
        self.files
            .insert(PathBuf::from(path), FileContent::Binary(data));
        self
    }
}

impl Default for TestStructure {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration options for serve.json
#[derive(Debug, Clone, Default)]
pub struct ServeJsonOptions {
    pub clean_urls: Option<bool>,
    pub trailing_slash: Option<bool>,
    pub etag: Option<bool>,
    pub directory_listing: Option<bool>,
    pub symlinks: Option<bool>,
    pub rewrites: Vec<serde_json::Value>,
    pub redirects: Vec<serde_json::Value>,
    pub headers: Vec<serde_json::Value>,
}

/// Configuration options for now.json
#[derive(Debug, Clone, Default)]
pub struct NowJsonOptions {
    pub clean_urls: Option<bool>,
    pub trailing_slash: Option<bool>,
    pub render_single: Option<bool>,
    pub etag: Option<bool>,
    pub directory_listing: Option<bool>,
    pub symlinks: Option<bool>,
}

/// Configuration options for package.json
#[derive(Debug, Clone, Default)]
pub struct PackageJsonOptions {
    pub name: Option<String>,
    pub version: Option<String>,
    pub clean_urls: Option<bool>,
    pub render_single: Option<bool>,
    pub etag: Option<bool>,
}

/// Test environment with multiple directories for configuration testing
#[derive(Debug)]
pub struct ConfigTestEnvironment {
    pub base_dir: PathBuf,
    pub public_dir: PathBuf,
    pub dist_dir: PathBuf,
    pub build_dir: PathBuf,
    pub static_dir: PathBuf,
}

/// POST test files collection
#[derive(Debug)]
pub struct PostTestFiles {
    pub index_html: PathBuf,
    pub sample_txt: PathBuf,
    pub sample_json: PathBuf,
    pub sample_png: PathBuf,
    pub large_file: PathBuf,
}

/// SSL/HTTPS test files collection
#[derive(Debug)]
pub struct SslTestFiles {
    pub index_html: PathBuf,
    pub api_json: PathBuf,
}

/// Advanced features test files collection
#[derive(Debug)]
pub struct AdvancedTestFiles {
    pub index_html: PathBuf,
    pub spa_dir: PathBuf,
    pub subdirectory: PathBuf,
    pub symlink_target: PathBuf,
    pub serve_json: PathBuf,
}