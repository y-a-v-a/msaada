// src/spa.rs
// Single Page Application support and advanced web features

use actix_files::NamedFile;
use actix_web::{
    web, HttpRequest, HttpResponse, Result,
};
use std::path::PathBuf;

/// SPA fallback handler that serves index.html for all non-API routes
/// (Kept for backward compatibility - use configurable_spa_handler for advanced features)
#[allow(dead_code)]
pub async fn spa_fallback_handler(
    req: HttpRequest,
    directory: web::Data<PathBuf>,
) -> Result<HttpResponse> {
    let path = req.path();
    
    // Use the should_use_spa_fallback function to determine if we should handle this route
    if !should_use_spa_fallback(path) {
        return Ok(HttpResponse::NotFound().finish());
    }
    
    // Serve index.html for SPA routes
    let index_path = directory.join("index.html");
    
    if index_path.exists() {
        match NamedFile::open(&index_path) {
            Ok(file) => {
                let response = file.into_response(&req);
                Ok(response.into())
            },
            Err(_) => Ok(HttpResponse::NotFound().finish()),
        }
    } else {
        Ok(HttpResponse::NotFound().body("index.html not found - required for SPA mode"))
    }
}

/// Advanced SPA fallback handler with URL processing based on configuration
pub async fn configurable_spa_handler(
    req: HttpRequest,
    directory: PathBuf,
    clean_urls: bool,
    trailing_slash: bool,
    rewrites: Vec<crate::config::Rewrite>,
) -> Result<HttpResponse> {
    let path = req.path();
    
    // First, apply URL rewrites if configured
    let processed_path = apply_url_rewrites(path, &rewrites);
    let processed_path = if clean_urls { apply_clean_urls(&processed_path) } else { processed_path };
    let processed_path = apply_trailing_slash(&processed_path, trailing_slash);
    
    // Use the should_use_spa_fallback function to determine if we should handle this route
    if !should_use_spa_fallback(&processed_path) {
        return Ok(HttpResponse::NotFound().finish());
    }
    
    // Serve index.html for SPA routes
    let index_path = directory.join("index.html");
    
    if index_path.exists() {
        match NamedFile::open(&index_path) {
            Ok(file) => {
                let response = file.into_response(&req);
                Ok(response.into())
            },
            Err(_) => Ok(HttpResponse::NotFound().finish()),
        }
    } else {
        Ok(HttpResponse::NotFound().body("index.html not found - required for SPA mode"))
    }
}

// Removed the complex 404 handler for now - SPA fallback is handled by default_service

/// Utility function to check if a route should use SPA fallback
pub fn should_use_spa_fallback(path: &str) -> bool {
    // Don't use SPA fallback for:
    // - API routes
    // - Static assets (files with extensions)
    // - Special routes starting with underscore
    // - Root path (/)
    
    if path == "/" {
        return false;
    }
    
    if path.starts_with("/api/") || 
       path.starts_with("/_") ||
       path.contains('.') {
        return false;
    }
    
    true
}

/// URL rewrite handler based on configuration
pub fn apply_url_rewrites(path: &str, rewrites: &[crate::config::Rewrite]) -> String {
    for rewrite in rewrites {
        // Simple pattern matching - in a full implementation you'd want
        // proper glob pattern matching
        if rewrite.source == "**" || path.starts_with(&rewrite.source) {
            return rewrite.destination.clone();
        }
        
        // Exact match
        if rewrite.source == path {
            return rewrite.destination.clone();
        }
        
        // Wildcard matching (simplified)
        if rewrite.source.ends_with("*") {
            let prefix = &rewrite.source[..rewrite.source.len() - 1];
            if path.starts_with(prefix) {
                return rewrite.destination.clone();
            }
        }
    }
    
    path.to_string()
}

/// Clean URLs handler - removes .html extension from URLs
pub fn apply_clean_urls(path: &str) -> String {
    if path.ends_with(".html") && path != "/index.html" {
        path[..path.len() - 5].to_string()
    } else {
        path.to_string()
    }
}

/// Add trailing slash if configured
pub fn apply_trailing_slash(path: &str, add_trailing_slash: bool) -> String {
    if add_trailing_slash && !path.ends_with('/') && !path.contains('.') {
        format!("{}/", path)
    } else if !add_trailing_slash && path.ends_with('/') && path != "/" {
        path[..path.len() - 1].to_string()
    } else {
        path.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Rewrite;

    #[test]
    fn test_should_use_spa_fallback() {
        // Should use SPA fallback
        assert!(should_use_spa_fallback("/about"));
        assert!(should_use_spa_fallback("/user/profile"));
        assert!(should_use_spa_fallback("/settings/account"));
        
        // Should NOT use SPA fallback
        assert!(!should_use_spa_fallback("/"));
        assert!(!should_use_spa_fallback("/api/users"));
        assert!(!should_use_spa_fallback("/_health"));
        assert!(!should_use_spa_fallback("/style.css"));
        assert!(!should_use_spa_fallback("/images/logo.png"));
        assert!(!should_use_spa_fallback("/js/app.min.js"));
    }

    #[test]
    fn test_apply_url_rewrites() {
        let rewrites = vec![
            Rewrite {
                source: "/old-path".to_string(),
                destination: "/new-path".to_string(),
            },
            Rewrite {
                source: "/api/*".to_string(),
                destination: "/v1/api/".to_string(),
            },
            Rewrite {
                source: "**".to_string(),
                destination: "/index.html".to_string(),
            },
        ];

        // Exact match
        assert_eq!(apply_url_rewrites("/old-path", &rewrites), "/new-path");
        
        // Wildcard match
        assert_eq!(apply_url_rewrites("/api/users", &rewrites), "/v1/api/");
        
        // Catch-all match
        assert_eq!(apply_url_rewrites("/some/random/path", &rewrites), "/index.html");
    }

    #[test]
    fn test_apply_clean_urls() {
        assert_eq!(apply_clean_urls("/about.html"), "/about");
        assert_eq!(apply_clean_urls("/contact.html"), "/contact");
        assert_eq!(apply_clean_urls("/index.html"), "/index.html"); // Keep index.html
        assert_eq!(apply_clean_urls("/style.css"), "/style.css"); // Not HTML
        assert_eq!(apply_clean_urls("/about"), "/about"); // Already clean
    }

    #[test]
    fn test_apply_trailing_slash() {
        // Add trailing slash
        assert_eq!(apply_trailing_slash("/about", true), "/about/");
        assert_eq!(apply_trailing_slash("/user/profile", true), "/user/profile/");
        assert_eq!(apply_trailing_slash("/", true), "/"); // Root unchanged
        assert_eq!(apply_trailing_slash("/style.css", true), "/style.css"); // Files unchanged
        
        // Remove trailing slash
        assert_eq!(apply_trailing_slash("/about/", false), "/about");
        assert_eq!(apply_trailing_slash("/user/profile/", false), "/user/profile");
        assert_eq!(apply_trailing_slash("/", false), "/"); // Root unchanged
        assert_eq!(apply_trailing_slash("/about", false), "/about"); // No slash to remove
    }
}