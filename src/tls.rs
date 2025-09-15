// src/tls.rs
// TLS/SSL certificate loading and configuration

use rustls::{Certificate, PrivateKey, ServerConfig};
use rustls_pemfile::{certs, pkcs8_private_keys, rsa_private_keys};
use std::fs::File;
use std::io::{self, BufReader};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub enum CertFormat {
    Pem,
    Pkcs12,
}

#[derive(Debug, Clone)]
pub struct TlsConfig {
    pub cert_path: PathBuf,
    pub key_path: Option<PathBuf>,
    pub passphrase_path: Option<PathBuf>,
    pub format: CertFormat,
}

#[derive(Debug)]
pub enum TlsError {
    IoError(io::Error),
    InvalidCertificate(String),
    InvalidPrivateKey(String),
    MissingPrivateKey,
    InvalidPassphrase(String),
    Pkcs12Error(String),
    ConfigError(String),
}

impl std::fmt::Display for TlsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TlsError::IoError(e) => write!(f, "IO error: {}", e),
            TlsError::InvalidCertificate(msg) => write!(f, "Invalid certificate: {}", msg),
            TlsError::InvalidPrivateKey(msg) => write!(f, "Invalid private key: {}", msg),
            TlsError::MissingPrivateKey => write!(f, "Private key is required for PEM certificates"),
            TlsError::InvalidPassphrase(msg) => write!(f, "Invalid passphrase: {}", msg),
            TlsError::Pkcs12Error(msg) => write!(f, "PKCS12 error: {}", msg),
            TlsError::ConfigError(msg) => write!(f, "TLS configuration error: {}", msg),
        }
    }
}

impl std::error::Error for TlsError {}

impl From<io::Error> for TlsError {
    fn from(err: io::Error) -> Self {
        TlsError::IoError(err)
    }
}

impl From<rustls::Error> for TlsError {
    fn from(err: rustls::Error) -> Self {
        TlsError::ConfigError(format!("Rustls error: {}", err))
    }
}

impl TlsConfig {
    /// Create TLS configuration from command line arguments
    pub fn from_args(
        cert_path: &str,
        key_path: Option<&str>,
        passphrase_path: Option<&str>,
    ) -> Result<Self, TlsError> {
        let cert_path = PathBuf::from(cert_path);
        
        // Detect certificate format based on file extension
        let format = Self::detect_format(&cert_path);
        
        // Validate argument combinations
        match format {
            CertFormat::Pem => {
                if key_path.is_none() {
                    return Err(TlsError::MissingPrivateKey);
                }
            }
            CertFormat::Pkcs12 => {
                // PKCS12 format contains both certificate and private key
                // key_path should be None for PKCS12
            }
        }
        
        Ok(TlsConfig {
            cert_path,
            key_path: key_path.map(PathBuf::from),
            passphrase_path: passphrase_path.map(PathBuf::from),
            format,
        })
    }
    
    /// Detect certificate format based on file extension
    fn detect_format(cert_path: &Path) -> CertFormat {
        if let Some(extension) = cert_path.extension().and_then(|s| s.to_str()) {
            match extension.to_lowercase().as_str() {
                "pfx" | "p12" => CertFormat::Pkcs12,
                _ => CertFormat::Pem,
            }
        } else {
            CertFormat::Pem
        }
    }
    
    /// Load server configuration for rustls
    pub async fn load_server_config(&self) -> Result<ServerConfig, TlsError> {
        match self.format {
            CertFormat::Pem => self.load_pem_config().await,
            CertFormat::Pkcs12 => self.load_pkcs12_config().await,
        }
    }
    
    /// Load PEM format certificates (separate cert and key files)
    async fn load_pem_config(&self) -> Result<ServerConfig, TlsError> {
        // Load certificate chain
        let cert_file = File::open(&self.cert_path)?;
        let mut cert_reader = BufReader::new(cert_file);
        let cert_chain = certs(&mut cert_reader)
            .map_err(|e| TlsError::InvalidCertificate(format!("Failed to parse certificates: {}", e)))?
            .into_iter()
            .map(Certificate)
            .collect::<Vec<_>>();
        
        if cert_chain.is_empty() {
            return Err(TlsError::InvalidCertificate("No certificates found in file".to_string()));
        }
        
        // Load private key
        let key_path = self.key_path.as_ref().unwrap(); // Already validated in from_args
        let key_file = File::open(key_path)?;
        let mut key_reader = BufReader::new(key_file);
        
        // Try PKCS8 first, then RSA
        let private_key = pkcs8_private_keys(&mut key_reader)
            .map_err(|e| TlsError::InvalidPrivateKey(format!("Failed to parse PKCS8 private key: {}", e)))?
            .into_iter()
            .map(PrivateKey)
            .next()
            .or_else(|| {
                // Reset file reader and try RSA format
                let key_file = File::open(key_path).ok()?;
                let mut key_reader = BufReader::new(key_file);
                rsa_private_keys(&mut key_reader)
                    .ok()?
                    .into_iter()
                    .map(PrivateKey)
                    .next()
            })
            .ok_or_else(|| TlsError::InvalidPrivateKey("No valid private key found".to_string()))?;
        
        // Create server configuration
        let config = ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(cert_chain, private_key)?;
        
        Ok(config)
    }
    
    /// Load PKCS12 format certificates (single file with cert and key)
    async fn load_pkcs12_config(&self) -> Result<ServerConfig, TlsError> {
        // Read PKCS12 file
        let p12_data = std::fs::read(&self.cert_path)?;
        
        // Read passphrase if provided
        let passphrase = if let Some(ref passphrase_path) = self.passphrase_path {
            std::fs::read_to_string(passphrase_path)
                .map_err(|e| TlsError::InvalidPassphrase(format!("Failed to read passphrase file: {}", e)))?
                .trim()
                .to_string()
        } else {
            String::new()
        };
        
        // Parse PKCS12
        let pfx = p12::PFX::parse(&p12_data)
            .map_err(|e| TlsError::Pkcs12Error(format!("Failed to parse PKCS12 file: {}", e)))?;
        
        // Verify MAC (password authentication)
        if !pfx.verify_mac(&passphrase) {
            return Err(TlsError::InvalidPassphrase("Invalid passphrase for PKCS12 file".to_string()));
        }
        
        // Extract certificates
        let cert_ders = pfx.cert_bags(&passphrase)
            .map_err(|e| TlsError::Pkcs12Error(format!("Failed to extract certificates: {}", e)))?;
        
        if cert_ders.is_empty() {
            return Err(TlsError::InvalidCertificate("No certificates found in PKCS12 file".to_string()));
        }
        
        let cert_chain: Vec<Certificate> = cert_ders
            .into_iter()
            .map(|cert_der| Certificate(cert_der))
            .collect();
        
        // Extract private keys
        let key_ders = pfx.key_bags(&passphrase)
            .map_err(|e| TlsError::Pkcs12Error(format!("Failed to extract private keys: {}", e)))?;
        
        if key_ders.is_empty() {
            return Err(TlsError::InvalidPrivateKey("No private key found in PKCS12 file".to_string()));
        }
        
        let private_key = PrivateKey(key_ders[0].clone());
        
        // Create server configuration
        let config = ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(cert_chain, private_key)?;
        
        Ok(config)
    }
}

/// Utility function to validate SSL certificate arguments
pub fn validate_ssl_args(
    cert: Option<&str>,
    key: Option<&str>,
    passphrase: Option<&str>,
) -> Result<Option<TlsConfig>, TlsError> {
    match cert {
        Some(cert_path) => {
            let config = TlsConfig::from_args(cert_path, key, passphrase)?;
            Ok(Some(config))
        }
        None => {
            if key.is_some() || passphrase.is_some() {
                return Err(TlsError::ConfigError(
                    "SSL key or passphrase provided without certificate".to_string()
                ));
            }
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_format() {
        assert!(matches!(TlsConfig::detect_format(Path::new("cert.pem")), CertFormat::Pem));
        assert!(matches!(TlsConfig::detect_format(Path::new("cert.crt")), CertFormat::Pem));
        assert!(matches!(TlsConfig::detect_format(Path::new("cert.pfx")), CertFormat::Pkcs12));
        assert!(matches!(TlsConfig::detect_format(Path::new("cert.p12")), CertFormat::Pkcs12));
        assert!(matches!(TlsConfig::detect_format(Path::new("cert")), CertFormat::Pem)); // Default
    }

    #[test]
    fn test_from_args_pem_valid() {
        let result = TlsConfig::from_args("cert.pem", Some("key.pem"), None);
        assert!(result.is_ok());
        
        let config = result.unwrap();
        assert_eq!(config.cert_path, PathBuf::from("cert.pem"));
        assert_eq!(config.key_path, Some(PathBuf::from("key.pem")));
        assert!(matches!(config.format, CertFormat::Pem));
    }

    #[test]
    fn test_from_args_pem_missing_key() {
        let result = TlsConfig::from_args("cert.pem", None, None);
        assert!(matches!(result, Err(TlsError::MissingPrivateKey)));
    }

    #[test]
    fn test_from_args_pkcs12_valid() {
        let result = TlsConfig::from_args("cert.pfx", None, Some("pass.txt"));
        assert!(result.is_ok());
        
        let config = result.unwrap();
        assert_eq!(config.cert_path, PathBuf::from("cert.pfx"));
        assert_eq!(config.key_path, None);
        assert_eq!(config.passphrase_path, Some(PathBuf::from("pass.txt")));
        assert!(matches!(config.format, CertFormat::Pkcs12));
    }

    #[test]
    fn test_validate_ssl_args_none() {
        let result = validate_ssl_args(None, None, None);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_validate_ssl_args_invalid_combination() {
        let result = validate_ssl_args(None, Some("key.pem"), None);
        assert!(matches!(result, Err(TlsError::ConfigError(_))));
    }

    #[test]
    fn test_validate_ssl_args_valid() {
        let result = validate_ssl_args(Some("cert.pem"), Some("key.pem"), None);
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[test]
    fn test_detect_format_edge_cases() {
        // Test various file extensions
        assert!(matches!(TlsConfig::detect_format(Path::new("cert.PFX")), CertFormat::Pkcs12)); // Uppercase
        assert!(matches!(TlsConfig::detect_format(Path::new("cert.P12")), CertFormat::Pkcs12)); // Uppercase
        assert!(matches!(TlsConfig::detect_format(Path::new("certificate.crt")), CertFormat::Pem));
        assert!(matches!(TlsConfig::detect_format(Path::new("key.key")), CertFormat::Pem));
        assert!(matches!(TlsConfig::detect_format(Path::new("cert.der")), CertFormat::Pem)); // DER defaults to PEM
        
        // No extension should default to PEM
        assert!(matches!(TlsConfig::detect_format(Path::new("mycert")), CertFormat::Pem));
        
        // Path with multiple dots
        assert!(matches!(TlsConfig::detect_format(Path::new("my.cert.pem")), CertFormat::Pem));
        assert!(matches!(TlsConfig::detect_format(Path::new("my.cert.pfx")), CertFormat::Pkcs12));
    }

    #[test]
    fn test_tls_config_validation_combinations() {
        // PEM with both cert and key - valid
        let result = TlsConfig::from_args("server.pem", Some("server.key"), None);
        assert!(result.is_ok());
        
        // PEM with cert, key, and passphrase - valid
        let result = TlsConfig::from_args("server.pem", Some("server.key"), Some("pass.txt"));
        assert!(result.is_ok());
        
        // PKCS12 with cert only - valid
        let result = TlsConfig::from_args("server.p12", None, None);
        assert!(result.is_ok());
        
        // PKCS12 with cert and passphrase - valid
        let result = TlsConfig::from_args("server.p12", None, Some("pass.txt"));
        assert!(result.is_ok());
        
        // PKCS12 with cert and key (should still work, key will be ignored)
        let result = TlsConfig::from_args("server.pfx", Some("ignored.key"), None);
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.key_path, Some(PathBuf::from("ignored.key")));
    }

    #[test]
    fn test_error_display_formatting() {
        let io_error = TlsError::IoError(std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"));
        assert!(io_error.to_string().contains("IO error"));
        
        let cert_error = TlsError::InvalidCertificate("bad cert".to_string());
        assert_eq!(cert_error.to_string(), "Invalid certificate: bad cert");
        
        let key_error = TlsError::InvalidPrivateKey("bad key".to_string());
        assert_eq!(key_error.to_string(), "Invalid private key: bad key");
        
        let missing_key = TlsError::MissingPrivateKey;
        assert_eq!(missing_key.to_string(), "Private key is required for PEM certificates");
        
        let passphrase_error = TlsError::InvalidPassphrase("wrong pass".to_string());
        assert_eq!(passphrase_error.to_string(), "Invalid passphrase: wrong pass");
        
        let pkcs12_error = TlsError::Pkcs12Error("p12 issue".to_string());
        assert_eq!(pkcs12_error.to_string(), "PKCS12 error: p12 issue");
        
        let config_error = TlsError::ConfigError("config issue".to_string());
        assert_eq!(config_error.to_string(), "TLS configuration error: config issue");
    }

    #[test]
    fn test_validate_ssl_args_edge_cases() {
        // Only passphrase provided (invalid)
        let result = validate_ssl_args(None, None, Some("pass.txt"));
        assert!(matches!(result, Err(TlsError::ConfigError(_))));
        
        // Only key provided (invalid)
        let result = validate_ssl_args(None, Some("key.pem"), None);
        assert!(matches!(result, Err(TlsError::ConfigError(_))));
        
        // Key and passphrase without cert (invalid)
        let result = validate_ssl_args(None, Some("key.pem"), Some("pass.txt"));
        assert!(matches!(result, Err(TlsError::ConfigError(_))));
    }

    #[test]
    fn test_tls_config_path_handling() {
        let config = TlsConfig::from_args("/absolute/path/cert.pem", Some("/absolute/path/key.pem"), None).unwrap();
        assert_eq!(config.cert_path, PathBuf::from("/absolute/path/cert.pem"));
        assert_eq!(config.key_path, Some(PathBuf::from("/absolute/path/key.pem")));
        
        let config = TlsConfig::from_args("relative/cert.pfx", None, Some("relative/pass.txt")).unwrap();
        assert_eq!(config.cert_path, PathBuf::from("relative/cert.pfx"));
        assert_eq!(config.passphrase_path, Some(PathBuf::from("relative/pass.txt")));
    }

    #[test]
    fn test_certificate_format_consistency() {
        // Ensure format detection is consistent with validation
        let pem_config = TlsConfig::from_args("cert.pem", Some("key.pem"), None).unwrap();
        assert!(matches!(pem_config.format, CertFormat::Pem));
        
        let p12_config = TlsConfig::from_args("cert.p12", None, Some("pass.txt")).unwrap();
        assert!(matches!(p12_config.format, CertFormat::Pkcs12));
        
        let pfx_config = TlsConfig::from_args("cert.pfx", None, None).unwrap();
        assert!(matches!(pfx_config.format, CertFormat::Pkcs12));
    }
}