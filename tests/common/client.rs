//! HTTP client testing utilities
//!
//! This module provides the TestClient struct and related functionality for
//! making HTTP requests during testing.
#![allow(dead_code)] // Different suites use different client helpers; silence per-module warnings.

use std::collections::HashMap;
use std::time::Duration;

use reqwest::{Client, Response};
use serde_json::Value;

/// HTTP testing client wrapper
pub struct TestClient {
    pub client: Client,
}

impl TestClient {
    /// Create a new test client
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        TestClient { client }
    }

    /// Perform a GET request
    pub async fn get(&self, url: &str) -> Result<Response, Box<dyn std::error::Error>> {
        let response = self.client.get(url).send().await?;
        Ok(response)
    }

    /// Perform a POST request with JSON body
    pub async fn post_json(
        &self,
        url: &str,
        body: &Value,
    ) -> Result<Response, Box<dyn std::error::Error>> {
        let response = self.client.post(url).json(body).send().await?;
        Ok(response)
    }

    /// Perform a POST request with form data
    pub async fn post_form(
        &self,
        url: &str,
        form: &HashMap<String, String>,
    ) -> Result<Response, Box<dyn std::error::Error>> {
        let response = self.client.post(url).form(form).send().await?;
        Ok(response)
    }

    /// Perform a POST request with multipart form data
    pub async fn post_multipart(
        &self,
        url: &str,
        form: reqwest::multipart::Form,
    ) -> Result<Response, Box<dyn std::error::Error>> {
        let response = self.client.post(url).multipart(form).send().await?;
        Ok(response)
    }

    /// Perform a HEAD request
    pub async fn head(&self, url: &str) -> Result<Response, Box<dyn std::error::Error>> {
        let response = self.client.head(url).send().await?;
        Ok(response)
    }

    /// Perform a POST request with plain text body
    pub async fn post_text(
        &self,
        url: &str,
        text: &str,
    ) -> Result<Response, Box<dyn std::error::Error>> {
        let response = self
            .client
            .post(url)
            .header("Content-Type", "text/plain")
            .body(text.to_string())
            .send()
            .await?;
        Ok(response)
    }

    /// Perform a POST request with binary data and custom content type
    pub async fn post_binary(
        &self,
        url: &str,
        data: Vec<u8>,
        content_type: &str,
    ) -> Result<Response, Box<dyn std::error::Error>> {
        let response = self
            .client
            .post(url)
            .header("Content-Type", content_type)
            .body(data)
            .send()
            .await?;
        Ok(response)
    }
}

impl Default for TestClient {
    fn default() -> Self {
        Self::new()
    }
}
