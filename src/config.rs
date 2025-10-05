// src/config.rs
// Configuration system for msaada

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rewrite {
    pub source: String,
    pub destination: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Redirect {
    pub source: String,
    pub destination: String,
    #[serde(rename = "type")]
    pub redirect_type: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderEntry {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Header {
    pub source: String,
    pub headers: Vec<HeaderEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Configuration {
    #[serde(default)]
    pub public: Option<String>,

    #[serde(default = "default_clean_urls")]
    pub clean_urls: bool,

    #[serde(default)]
    pub rewrites: Vec<Rewrite>,

    #[serde(default)]
    pub redirects: Vec<Redirect>,

    #[serde(default)]
    pub headers: Vec<Header>,

    #[serde(default = "default_directory_listing")]
    pub directory_listing: bool,

    #[serde(default)]
    pub unlisted: Vec<String>,

    #[serde(default)]
    pub trailing_slash: bool,

    #[serde(default)]
    pub render_single: bool,

    #[serde(default)]
    pub symlinks: bool,

    #[serde(default = "default_etag")]
    pub etag: bool,
}

fn default_clean_urls() -> bool {
    false
}
fn default_directory_listing() -> bool {
    true
}
fn default_etag() -> bool {
    true
}

impl Default for Configuration {
    fn default() -> Self {
        Configuration {
            public: None,
            clean_urls: default_clean_urls(),
            rewrites: Vec::new(),
            redirects: Vec::new(),
            headers: Vec::new(),
            directory_listing: default_directory_listing(),
            unlisted: Vec::new(),
            trailing_slash: false,
            render_single: false,
            symlinks: false,
            etag: default_etag(),
        }
    }
}

#[derive(Debug)]
pub enum ConfigError {
    FileNotFound(String),
    ParseError(String),
    ValidationError(String),
    IoError(std::io::Error),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::FileNotFound(path) => write!(f, "Configuration file not found: {}", path),
            ConfigError::ParseError(msg) => write!(f, "Failed to parse configuration: {}", msg),
            ConfigError::ValidationError(msg) => {
                write!(f, "Configuration validation failed: {}", msg)
            }
            ConfigError::IoError(err) => write!(f, "IO error: {}", err),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<std::io::Error> for ConfigError {
    fn from(err: std::io::Error) -> Self {
        ConfigError::IoError(err)
    }
}

pub struct ConfigLoader {
    current_dir: PathBuf,
    serve_dir: PathBuf,
}

impl ConfigLoader {
    pub fn new(current_dir: PathBuf, serve_dir: PathBuf) -> Self {
        Self {
            current_dir,
            serve_dir,
        }
    }

    pub fn load_configuration(
        &self,
        custom_config_path: Option<&str>,
    ) -> Result<Configuration, ConfigError> {
        let mut config = Configuration::default();

        // Define the configuration files to check in order of priority
        let config_files = if let Some(custom_path) = custom_config_path {
            vec![custom_path.to_string()]
        } else {
            vec![
                "serve.json".to_string(),
                "now.json".to_string(),
                "package.json".to_string(),
            ]
        };

        // Try to load configuration from the files
        for file_name in config_files {
            // If custom config path is provided, use it directly (it may be absolute)
            // Otherwise, join with serve_dir for relative paths
            let config_path = if custom_config_path.is_some() {
                PathBuf::from(&file_name)
            } else {
                self.serve_dir.join(&file_name)
            };

            if !config_path.exists() {
                if custom_config_path.is_some() {
                    return Err(ConfigError::FileNotFound(
                        config_path.to_string_lossy().to_string(),
                    ));
                }
                continue;
            }

            let contents = fs::read_to_string(&config_path)?;

            log::info!("Loading configuration from: {}", config_path.display());

            // Extract just the filename for matching (handles both relative and absolute paths)
            let config_filename = config_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");

            match config_filename {
                "serve.json" => {
                    config = serde_json::from_str(&contents)
                        .map_err(|e| ConfigError::ParseError(format!("serve.json: {}", e)))?;
                    log::info!(
                        "Parsed serve.json successfully, rewrites: {}",
                        config.rewrites.len()
                    );
                }
                "now.json" => {
                    #[derive(Deserialize)]
                    struct NowConfig {
                        now: Option<NowStatic>,
                    }

                    #[derive(Deserialize)]
                    struct NowStatic {
                        #[serde(rename = "static")]
                        static_config: Option<Configuration>,
                    }

                    let now_config: NowConfig = serde_json::from_str(&contents)
                        .map_err(|e| ConfigError::ParseError(format!("now.json: {}", e)))?;

                    if let Some(now) = now_config.now {
                        if let Some(static_config) = now.static_config {
                            config = static_config;
                        }
                    }

                    log::warn!(
                        "The config file `now.json` is deprecated. Please use `serve.json`."
                    );
                }
                "package.json" => {
                    #[derive(Deserialize)]
                    struct PackageJson {
                        #[serde(rename = "static")]
                        static_config: Option<Configuration>,
                    }

                    let package_json: PackageJson = serde_json::from_str(&contents)
                        .map_err(|e| ConfigError::ParseError(format!("package.json: {}", e)))?;

                    if let Some(static_config) = package_json.static_config {
                        config = static_config;
                    }

                    log::warn!("The config file `package.json` (static section) is deprecated. Please use `serve.json`.");
                }
                _ => {}
            }

            break; // Found and loaded a config file, stop looking
        }

        // Resolve the public directory path relative to the serve directory
        if let Some(ref public_dir) = config.public {
            let public_path = if Path::new(public_dir).is_absolute() {
                PathBuf::from(public_dir)
            } else {
                self.serve_dir.join(public_dir)
            };

            // Make it relative to current directory for actix-files
            let relative_path = public_path
                .strip_prefix(&self.current_dir)
                .unwrap_or(&public_path);

            config.public = Some(relative_path.to_string_lossy().to_string());
        } else {
            // Default to the serve directory
            let relative_path = self
                .serve_dir
                .strip_prefix(&self.current_dir)
                .unwrap_or(&self.serve_dir);

            config.public = Some(relative_path.to_string_lossy().to_string());
        }

        // Validate the configuration
        self.validate_config(&config)?;

        Ok(config)
    }

    fn validate_config(&self, config: &Configuration) -> Result<(), ConfigError> {
        // Validate public directory exists
        if let Some(ref public_dir) = config.public {
            let public_path = if Path::new(public_dir).is_absolute() {
                PathBuf::from(public_dir)
            } else {
                // Public directory should be relative to serve_dir, not current_dir
                self.serve_dir.join(public_dir)
            };

            if !public_path.exists() {
                return Err(ConfigError::ValidationError(format!(
                    "Public directory does not exist: {}",
                    public_path.display()
                )));
            }
        }

        // Validate rewrite sources are valid patterns
        for rewrite in &config.rewrites {
            if rewrite.source.is_empty() || rewrite.destination.is_empty() {
                return Err(ConfigError::ValidationError(
                    "Rewrite source and destination cannot be empty".to_string(),
                ));
            }
        }

        // Validate redirect types are valid HTTP status codes
        for redirect in &config.redirects {
            if !(300..400).contains(&redirect.redirect_type) {
                return Err(ConfigError::ValidationError(format!(
                    "Invalid redirect status code: {}",
                    redirect.redirect_type
                )));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_default_configuration() {
        let config = Configuration::default();
        assert!(!config.clean_urls);
        assert!(config.directory_listing);
        assert!(config.etag);
        assert_eq!(config.rewrites.len(), 0);
    }

    #[test]
    fn test_load_serve_json() {
        let temp_dir = TempDir::new().unwrap();
        let serve_dir = temp_dir.path().to_path_buf();

        // Create the public directory that will be referenced in the config
        let public_dir = serve_dir.join("public");
        fs::create_dir_all(&public_dir).unwrap();

        let config_content = r#"{
            "public": "public/",
            "cleanUrls": true,
            "rewrites": [
                {"source": "**", "destination": "/index.html"}
            ]
        }"#;

        fs::write(serve_dir.join("serve.json"), config_content).unwrap();

        let loader = ConfigLoader::new(temp_dir.path().to_path_buf(), serve_dir);
        let config = loader.load_configuration(None).unwrap();

        assert!(config.public.is_some());
        assert!(config.clean_urls);
        assert_eq!(config.rewrites.len(), 1);
        assert_eq!(config.rewrites[0].source, "**");
    }

    #[test]
    fn test_load_nonexistent_config() {
        let temp_dir = TempDir::new().unwrap();
        let serve_dir = temp_dir.path().to_path_buf();

        let loader = ConfigLoader::new(temp_dir.path().to_path_buf(), serve_dir);
        let config = loader.load_configuration(None).unwrap();

        // Should load default configuration
        assert!(!config.clean_urls);
        assert!(config.directory_listing);
    }

    #[test]
    fn test_custom_config_file_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let serve_dir = temp_dir.path().to_path_buf();

        let loader = ConfigLoader::new(temp_dir.path().to_path_buf(), serve_dir);
        let result = loader.load_configuration(Some("nonexistent.json"));

        assert!(matches!(result, Err(ConfigError::FileNotFound(_))));
    }

    #[test]
    fn test_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        let serve_dir = temp_dir.path().to_path_buf();

        fs::write(serve_dir.join("serve.json"), "{ invalid json }").unwrap();

        let loader = ConfigLoader::new(temp_dir.path().to_path_buf(), serve_dir);
        let result = loader.load_configuration(None);

        assert!(matches!(result, Err(ConfigError::ParseError(_))));
    }

    #[test]
    fn test_configuration_precedence_serve_json_over_package_json() {
        let temp_dir = TempDir::new().unwrap();
        let serve_dir = temp_dir.path().to_path_buf();

        // Create public directory
        let public_dir = serve_dir.join("public");
        fs::create_dir_all(&public_dir).unwrap();

        // Create both serve.json and package.json
        let serve_config = r#"{
            "cleanUrls": true,
            "etag": false
        }"#;

        let package_config = r#"{
            "static": {
                "cleanUrls": false,
                "etag": true
            }
        }"#;

        fs::write(serve_dir.join("serve.json"), serve_config).unwrap();
        fs::write(serve_dir.join("package.json"), package_config).unwrap();

        let loader = ConfigLoader::new(temp_dir.path().to_path_buf(), serve_dir);
        let config = loader.load_configuration(None).unwrap();

        // serve.json should take precedence
        assert!(config.clean_urls);
        assert!(!config.etag);
    }

    #[test]
    fn test_now_json_fallback_with_deprecation_warning() {
        let temp_dir = TempDir::new().unwrap();
        let serve_dir = temp_dir.path().to_path_buf();

        // Create public directory
        let public_dir = serve_dir.join("dist");
        fs::create_dir_all(&public_dir).unwrap();

        let now_config = r#"{
            "now": {
                "static": {
                    "public": "dist",
                    "cleanUrls": true
                }
            }
        }"#;

        fs::write(serve_dir.join("now.json"), now_config).unwrap();

        let loader = ConfigLoader::new(temp_dir.path().to_path_buf(), serve_dir);
        let config = loader.load_configuration(None).unwrap();

        assert!(config.clean_urls);
        assert!(config.public.is_some());
    }

    #[test]
    fn test_package_json_static_section() {
        let temp_dir = TempDir::new().unwrap();
        let serve_dir = temp_dir.path().to_path_buf();

        // Create public directory
        let public_dir = serve_dir.join("build");
        fs::create_dir_all(&public_dir).unwrap();

        let package_config = r#"{
            "name": "my-app",
            "version": "1.0.0",
            "static": {
                "public": "build",
                "renderSingle": true,
                "symlinks": true
            }
        }"#;

        fs::write(serve_dir.join("package.json"), package_config).unwrap();

        let loader = ConfigLoader::new(temp_dir.path().to_path_buf(), serve_dir);
        let config = loader.load_configuration(None).unwrap();

        assert!(config.render_single);
        assert!(config.symlinks);
        assert!(config.public.is_some());
    }

    #[test]
    fn test_validation_invalid_redirect_status_codes() {
        let temp_dir = TempDir::new().unwrap();
        let serve_dir = temp_dir.path().to_path_buf();

        let config_content = r#"{
            "redirects": [
                {"source": "/old", "destination": "/new", "type": 200},
                {"source": "/bad", "destination": "/good", "type": 400}
            ]
        }"#;

        fs::write(serve_dir.join("serve.json"), config_content).unwrap();

        let loader = ConfigLoader::new(temp_dir.path().to_path_buf(), serve_dir);
        let result = loader.load_configuration(None);

        assert!(matches!(result, Err(ConfigError::ValidationError(_))));
    }

    #[test]
    fn test_validation_empty_rewrite_rules() {
        let temp_dir = TempDir::new().unwrap();
        let serve_dir = temp_dir.path().to_path_buf();

        let config_content = r#"{
            "rewrites": [
                {"source": "", "destination": "/index.html"},
                {"source": "/api/*", "destination": ""}
            ]
        }"#;

        fs::write(serve_dir.join("serve.json"), config_content).unwrap();

        let loader = ConfigLoader::new(temp_dir.path().to_path_buf(), serve_dir);
        let result = loader.load_configuration(None);

        assert!(matches!(result, Err(ConfigError::ValidationError(_))));
    }

    #[test]
    fn test_validation_nonexistent_public_directory() {
        let temp_dir = TempDir::new().unwrap();
        let serve_dir = temp_dir.path().to_path_buf();

        let config_content = r#"{
            "public": "nonexistent-dir"
        }"#;

        fs::write(serve_dir.join("serve.json"), config_content).unwrap();

        let loader = ConfigLoader::new(temp_dir.path().to_path_buf(), serve_dir);
        let result = loader.load_configuration(None);

        assert!(matches!(result, Err(ConfigError::ValidationError(_))));
    }

    #[test]
    fn test_absolute_path_public_directory() {
        let temp_dir = TempDir::new().unwrap();
        let serve_dir = temp_dir.path().to_path_buf();

        // Create absolute path public directory
        let abs_public_dir = temp_dir.path().join("absolute_public");
        fs::create_dir_all(&abs_public_dir).unwrap();

        let config_content = format!(
            r#"{{
            "public": "{}"
        }}"#,
            abs_public_dir.to_string_lossy()
        );

        fs::write(serve_dir.join("serve.json"), config_content).unwrap();

        let loader = ConfigLoader::new(temp_dir.path().to_path_buf(), serve_dir);
        let config = loader.load_configuration(None).unwrap();

        assert!(config.public.is_some());
        // Should handle absolute paths correctly
        assert!(config.public.unwrap().contains("absolute_public"));
    }

    #[test]
    fn test_complex_configuration_with_all_options() {
        let temp_dir = TempDir::new().unwrap();
        let serve_dir = temp_dir.path().to_path_buf();

        // Create public directory
        let public_dir = serve_dir.join("dist");
        fs::create_dir_all(&public_dir).unwrap();

        let config_content = r#"{
            "public": "dist",
            "cleanUrls": true,
            "trailingSlash": true,
            "renderSingle": true,
            "symlinks": true,
            "etag": false,
            "directoryListing": false,
            "rewrites": [
                {"source": "/api/*", "destination": "/api/index.html"},
                {"source": "**", "destination": "/index.html"}
            ],
            "redirects": [
                {"source": "/old-api/*", "destination": "/api/", "type": 301},
                {"source": "/legacy", "destination": "/", "type": 302}
            ],
            "headers": [
                {
                    "source": "**/*.@(jpg|jpeg|png|gif)",
                    "headers": [
                        {"key": "Cache-Control", "value": "max-age=86400"},
                        {"key": "X-Content-Type-Options", "value": "nosniff"}
                    ]
                }
            ],
            "unlisted": [
                "*.log",
                "private/*"
            ]
        }"#;

        fs::write(serve_dir.join("serve.json"), config_content).unwrap();

        let loader = ConfigLoader::new(temp_dir.path().to_path_buf(), serve_dir);
        let config = loader.load_configuration(None).unwrap();

        // Verify all options are loaded correctly
        assert!(config.public.is_some());
        assert!(config.clean_urls);
        assert!(config.trailing_slash);
        assert!(config.render_single);
        assert!(config.symlinks);
        assert!(!config.etag);
        assert!(!config.directory_listing);
        assert_eq!(config.rewrites.len(), 2);
        assert_eq!(config.redirects.len(), 2);
        assert_eq!(config.headers.len(), 1);
        assert_eq!(config.headers[0].headers.len(), 2);
        assert_eq!(config.unlisted.len(), 2);

        // Verify redirect status codes
        assert_eq!(config.redirects[0].redirect_type, 301);
        assert_eq!(config.redirects[1].redirect_type, 302);
    }

    #[test]
    fn test_malformed_now_json_structure() {
        let temp_dir = TempDir::new().unwrap();
        let serve_dir = temp_dir.path().to_path_buf();

        // Create malformed now.json (missing "now" wrapper)
        let malformed_config = r#"{
            "static": {
                "cleanUrls": true
            }
        }"#;

        fs::write(serve_dir.join("now.json"), malformed_config).unwrap();

        let loader = ConfigLoader::new(temp_dir.path().to_path_buf(), serve_dir);
        let config = loader.load_configuration(None).unwrap();

        // Should fall back to default configuration
        assert!(!config.clean_urls); // Default value
    }

    #[test]
    fn test_valid_redirect_status_codes() {
        let temp_dir = TempDir::new().unwrap();
        let serve_dir = temp_dir.path().to_path_buf();

        let config_content = r#"{
            "redirects": [
                {"source": "/moved", "destination": "/new", "type": 301},
                {"source": "/temp", "destination": "/temporary", "type": 302},
                {"source": "/see-other", "destination": "/other", "type": 303},
                {"source": "/perm", "destination": "/permanent", "type": 308}
            ]
        }"#;

        fs::write(serve_dir.join("serve.json"), config_content).unwrap();

        let loader = ConfigLoader::new(temp_dir.path().to_path_buf(), serve_dir);
        let config = loader.load_configuration(None).unwrap();

        assert_eq!(config.redirects.len(), 4);
        assert_eq!(config.redirects[0].redirect_type, 301);
        assert_eq!(config.redirects[1].redirect_type, 302);
        assert_eq!(config.redirects[2].redirect_type, 303);
        assert_eq!(config.redirects[3].redirect_type, 308);
    }
}
