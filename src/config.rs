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

fn default_clean_urls() -> bool { false }
fn default_directory_listing() -> bool { true }
fn default_etag() -> bool { true }

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
            ConfigError::ValidationError(msg) => write!(f, "Configuration validation failed: {}", msg),
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
        Self { current_dir, serve_dir }
    }

    pub fn load_configuration(&self, custom_config_path: Option<&str>) -> Result<Configuration, ConfigError> {
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
            let config_path = self.serve_dir.join(&file_name);
            
            if !config_path.exists() {
                if custom_config_path.is_some() {
                    return Err(ConfigError::FileNotFound(config_path.to_string_lossy().to_string()));
                }
                continue;
            }

            let contents = fs::read_to_string(&config_path)?;
            
            match file_name.as_str() {
                "serve.json" => {
                    config = serde_json::from_str(&contents)
                        .map_err(|e| ConfigError::ParseError(format!("serve.json: {}", e)))?;
                },
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
                    
                    log::warn!("The config file `now.json` is deprecated. Please use `serve.json`.");
                },
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
                },
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
            let relative_path = self.serve_dir
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
                self.current_dir.join(public_dir)
            };
            
            if !public_path.exists() {
                return Err(ConfigError::ValidationError(
                    format!("Public directory does not exist: {}", public_path.display())
                ));
            }
        }

        // Validate rewrite sources are valid patterns
        for rewrite in &config.rewrites {
            if rewrite.source.is_empty() || rewrite.destination.is_empty() {
                return Err(ConfigError::ValidationError(
                    "Rewrite source and destination cannot be empty".to_string()
                ));
            }
        }

        // Validate redirect types are valid HTTP status codes
        for redirect in &config.redirects {
            if !(300..400).contains(&redirect.redirect_type) {
                return Err(ConfigError::ValidationError(
                    format!("Invalid redirect status code: {}", redirect.redirect_type)
                ));
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
        assert_eq!(config.clean_urls, false);
        assert_eq!(config.directory_listing, true);
        assert_eq!(config.etag, true);
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
        assert_eq!(config.clean_urls, true);
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
        assert_eq!(config.clean_urls, false);
        assert_eq!(config.directory_listing, true);
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
}