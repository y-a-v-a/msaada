//! SSL/TLS testing utilities
//!
//! This module provides utilities for generating test certificates and
//! configuring HTTPS clients for testing SSL functionality.

use std::time::Duration;

use reqwest::Client;
use tempfile::NamedTempFile;

/// SSL/TLS testing utilities
pub struct SslTestHelper;

impl SslTestHelper {
    /// Generate a self-signed certificate for testing
    pub fn generate_test_certificate() -> Result<(String, String), Box<dyn std::error::Error>> {
        use rcgen::{Certificate, CertificateParams};

        let mut params = CertificateParams::new(vec!["localhost".to_string()]);
        params.alg = &rcgen::PKCS_ECDSA_P256_SHA256;

        let cert = Certificate::from_params(params)?;
        let cert_pem = cert.serialize_pem()?;
        let key_pem = cert.serialize_private_key_pem();

        Ok((cert_pem, key_pem))
    }

    /// Create temporary certificate files for testing
    pub fn create_temp_cert_files(
    ) -> Result<(NamedTempFile, NamedTempFile), Box<dyn std::error::Error>> {
        let (cert_pem, key_pem) = Self::generate_test_certificate()?;

        let mut cert_file = NamedTempFile::new()?;
        std::io::Write::write_all(&mut cert_file, cert_pem.as_bytes())?;

        let mut key_file = NamedTempFile::new()?;
        std::io::Write::write_all(&mut key_file, key_pem.as_bytes())?;

        Ok((cert_file, key_file))
    }

    /// Create HTTPS client that accepts self-signed certificates
    pub fn create_https_client() -> Result<Client, Box<dyn std::error::Error>> {
        let client = Client::builder()
            .danger_accept_invalid_certs(true)
            .timeout(Duration::from_secs(30))
            .build()?;

        Ok(client)
    }

    /// Generate PKCS12 certificate with passphrase for testing
    /// Uses OpenSSL command to convert PEM to PKCS12 format
    pub fn generate_pkcs12_with_openssl(
        passphrase: &str,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        use std::process::Command;

        // First create temporary PEM files
        let (cert_file, key_file) = Self::create_temp_cert_files()?;

        // Create temporary PKCS12 file
        let p12_file = NamedTempFile::new()?;

        // Use OpenSSL to convert PEM to PKCS12
        let output = Command::new("openssl")
            .arg("pkcs12")
            .arg("-export")
            .arg("-out")
            .arg(p12_file.path())
            .arg("-inkey")
            .arg(key_file.path())
            .arg("-in")
            .arg(cert_file.path())
            .arg("-passout")
            .arg(format!("pass:{}", passphrase))
            .output()?;

        if !output.status.success() {
            return Err(format!(
                "OpenSSL PKCS12 conversion failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }

        // Read PKCS12 bytes
        let p12_bytes = std::fs::read(p12_file.path())?;

        Ok(p12_bytes)
    }

    /// Create temporary PKCS12 file and passphrase file for testing
    pub fn create_temp_pkcs12_files(
        passphrase: &str,
    ) -> Result<(NamedTempFile, NamedTempFile), Box<dyn std::error::Error>> {
        // Generate PKCS12 bytes using OpenSSL
        let p12_bytes = Self::generate_pkcs12_with_openssl(passphrase)?;

        // Write to temporary PKCS12 file
        let mut p12_file = NamedTempFile::new()?;
        std::io::Write::write_all(&mut p12_file, &p12_bytes)?;

        // Write passphrase to temporary file
        let mut pass_file = NamedTempFile::new()?;
        std::io::Write::write_all(&mut pass_file, passphrase.as_bytes())?;

        Ok((p12_file, pass_file))
    }
}