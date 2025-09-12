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
    use tempfile::TempDir;
    use std::fs;

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
}