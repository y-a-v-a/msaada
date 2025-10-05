//! Response validation and assertion utilities
//!
//! This module provides traits and helpers for validating HTTP responses
//! and making assertions in tests.

use reqwest::{Response, StatusCode};

/// Response validation helpers
pub trait ResponseAssertions {
    /// Assert response has expected status code
    fn assert_status(&self, expected: StatusCode) -> &Self;

    /// Assert response contains expected header
    fn assert_header(&self, name: &str, expected: &str) -> &Self;

    /// Assert response content type
    fn assert_content_type(&self, expected: &str) -> &Self;

    /// Get response text for further assertions
    async fn text_for_assertions(self) -> Result<String, Box<dyn std::error::Error>>;

    /// Get response JSON for further assertions
    async fn json_for_assertions<T: serde::de::DeserializeOwned>(
        self,
    ) -> Result<T, Box<dyn std::error::Error>>;
}

impl ResponseAssertions for Response {
    fn assert_status(&self, expected: StatusCode) -> &Self {
        assert_eq!(
            self.status(),
            expected,
            "Expected status {}, got {}",
            expected,
            self.status()
        );
        self
    }

    fn assert_header(&self, name: &str, expected: &str) -> &Self {
        let header_value = self
            .headers()
            .get(name)
            .unwrap_or_else(|| panic!("Header '{}' not found", name))
            .to_str()
            .unwrap_or_else(|_| panic!("Header '{}' contains invalid characters", name));

        assert_eq!(
            header_value, expected,
            "Expected header '{}' to be '{}', got '{}'",
            name, expected, header_value
        );
        self
    }

    fn assert_content_type(&self, expected: &str) -> &Self {
        self.assert_header("content-type", expected)
    }

    async fn text_for_assertions(self) -> Result<String, Box<dyn std::error::Error>> {
        Ok(self.text().await?)
    }

    async fn json_for_assertions<T: serde::de::DeserializeOwned>(
        self,
    ) -> Result<T, Box<dyn std::error::Error>> {
        Ok(self.json().await?)
    }
}
