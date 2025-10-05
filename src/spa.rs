// src/spa.rs
// Single Page Application support and advanced web features

use actix_files::NamedFile;
use actix_web::{web, HttpRequest, HttpResponse, Result};
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
                Ok(response)
            }
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
    let processed_path = if clean_urls {
        apply_clean_urls(&processed_path)
    } else {
        processed_path
    };
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
                Ok(response)
            }
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

    if path.starts_with("/api/") || path.starts_with("/_") || path.contains('.') {
        return false;
    }

    true
}

/// URL rewrite handler based on configuration
pub fn apply_url_rewrites(path: &str, rewrites: &[crate::config::Rewrite]) -> String {
    for rewrite in rewrites {
        // Exact match - highest priority
        if rewrite.source == path {
            return rewrite.destination.clone();
        }

        // Wildcard matching: /api/* matches /api/anything
        if rewrite.source.ends_with("/*") {
            let prefix = &rewrite.source[..rewrite.source.len() - 2]; // Remove "/*"
            if path.starts_with(prefix) && (path.len() == prefix.len() || path[prefix.len()..].starts_with('/')) {
                return rewrite.destination.clone();
            }
        }

        // Wildcard matching: /api/(.*)  - regex-style pattern
        if rewrite.source.contains("(.*)") {
            let prefix = rewrite.source.split("(.*)").next().unwrap();
            if path.starts_with(prefix) {
                return rewrite.destination.clone();
            }
        }

        // Catch-all pattern
        if rewrite.source == "**" {
            return rewrite.destination.clone();
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
        assert_eq!(
            apply_url_rewrites("/some/random/path", &rewrites),
            "/index.html"
        );
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
        assert_eq!(
            apply_trailing_slash("/user/profile", true),
            "/user/profile/"
        );
        assert_eq!(apply_trailing_slash("/", true), "/"); // Root unchanged
        assert_eq!(apply_trailing_slash("/style.css", true), "/style.css"); // Files unchanged

        // Remove trailing slash
        assert_eq!(apply_trailing_slash("/about/", false), "/about");
        assert_eq!(
            apply_trailing_slash("/user/profile/", false),
            "/user/profile"
        );
        assert_eq!(apply_trailing_slash("/", false), "/"); // Root unchanged
        assert_eq!(apply_trailing_slash("/about", false), "/about"); // No slash to remove
    }

    #[test]
    fn test_complex_url_rewrite_patterns() {
        let rewrites = vec![
            Rewrite {
                source: "/api/v1/*".to_string(),
                destination: "/api/v2/".to_string(),
            },
            Rewrite {
                source: "/legacy/*".to_string(),
                destination: "/modern/".to_string(),
            },
            Rewrite {
                source: "/docs/(.*)".to_string(), // Regex-style (simplified)
                destination: "/documentation/$1".to_string(),
            },
            Rewrite {
                source: "/app/**".to_string(), // Double wildcard
                destination: "/application/".to_string(),
            },
        ];

        // Test wildcard matching
        assert_eq!(apply_url_rewrites("/api/v1/users", &rewrites), "/api/v2/");
        assert_eq!(
            apply_url_rewrites("/api/v1/posts/123", &rewrites),
            "/api/v2/"
        );
        assert_eq!(
            apply_url_rewrites("/legacy/dashboard", &rewrites),
            "/modern/"
        );

        // Test double wildcard - the current implementation doesn't support ** in the middle
        assert_eq!(
            apply_url_rewrites("/app/deep/nested/path", &rewrites),
            "/app/deep/nested/path"
        );

        // Test no match
        assert_eq!(apply_url_rewrites("/nomatch", &rewrites), "/nomatch");
    }

    #[test]
    fn test_rewrite_precedence_order() {
        let rewrites = vec![
            Rewrite {
                source: "/api/specific".to_string(),
                destination: "/specific-endpoint".to_string(),
            },
            Rewrite {
                source: "/api/*".to_string(),
                destination: "/general-api/".to_string(),
            },
            Rewrite {
                source: "**".to_string(),
                destination: "/catch-all".to_string(),
            },
        ];

        // Specific match should take precedence
        assert_eq!(
            apply_url_rewrites("/api/specific", &rewrites),
            "/specific-endpoint"
        );

        // Wildcard match
        assert_eq!(apply_url_rewrites("/api/users", &rewrites), "/general-api/");

        // Catch-all match
        assert_eq!(
            apply_url_rewrites("/something/else", &rewrites),
            "/catch-all"
        );
    }

    #[test]
    fn test_should_use_spa_fallback_edge_cases() {
        // Test various file extensions
        assert!(!should_use_spa_fallback("/script.js"));
        assert!(!should_use_spa_fallback("/style.css"));
        assert!(!should_use_spa_fallback("/image.png"));
        assert!(!should_use_spa_fallback("/document.pdf"));
        assert!(!should_use_spa_fallback("/data.json"));
        assert!(!should_use_spa_fallback("/font.woff2"));
        assert!(!should_use_spa_fallback("/video.mp4"));

        // Test API variations
        assert!(should_use_spa_fallback("/api")); // Just /api without trailing slash should use fallback
        assert!(!should_use_spa_fallback("/api/v1/users"));
        assert!(!should_use_spa_fallback("/api/graphql"));
        assert!(should_use_spa_fallback("/application")); // Not /api/

        // Test underscore prefixed paths
        assert!(!should_use_spa_fallback("/_health"));
        assert!(!should_use_spa_fallback("/_next/static/chunks/main.js"));
        assert!(!should_use_spa_fallback("/_admin/login"));
        assert!(should_use_spa_fallback("/admin/login")); // No underscore

        // Test complex paths
        assert!(should_use_spa_fallback("/user/profile/settings"));
        assert!(should_use_spa_fallback("/dashboard"));
        assert!(should_use_spa_fallback("/products/category/electronics"));

        // Test root and empty paths
        assert!(!should_use_spa_fallback("/"));
        assert!(should_use_spa_fallback("")); // Empty string doesn't match any exclude rules
    }

    #[test]
    fn test_clean_urls_edge_cases() {
        // Standard cases
        assert_eq!(apply_clean_urls("/page.html"), "/page");
        assert_eq!(apply_clean_urls("/nested/page.html"), "/nested/page");

        // Keep only root index.html
        assert_eq!(apply_clean_urls("/index.html"), "/index.html");
        assert_eq!(apply_clean_urls("/folder/index.html"), "/folder/index");

        // Non-HTML files unchanged
        assert_eq!(apply_clean_urls("/style.css"), "/style.css");
        assert_eq!(apply_clean_urls("/script.js"), "/script.js");
        assert_eq!(apply_clean_urls("/data.json"), "/data.json");

        // Files without extensions
        assert_eq!(apply_clean_urls("/about"), "/about");
        assert_eq!(apply_clean_urls("/contact"), "/contact");

        // Edge cases with multiple dots
        assert_eq!(apply_clean_urls("/my.page.html"), "/my.page");
        assert_eq!(apply_clean_urls("/file.backup.html"), "/file.backup");

        // Empty and root paths
        assert_eq!(apply_clean_urls("/"), "/");
        assert_eq!(apply_clean_urls(""), "");
    }

    #[test]
    fn test_trailing_slash_with_query_params() {
        // Note: Current implementation doesn't handle query params,
        // but these tests document expected behavior

        // Paths with query parameters (current behavior)
        assert_eq!(
            apply_trailing_slash("/search?q=test", true),
            "/search?q=test/"
        );
        assert_eq!(
            apply_trailing_slash("/api/data?format=json", false),
            "/api/data?format=json"
        );

        // This shows the limitation - ideally we'd want:
        // "/search?q=test" -> "/search/?q=test" when adding slash
        // But current implementation treats ? as part of the path
    }

    #[test]
    fn test_url_processing_chain() {
        let rewrites = vec![Rewrite {
            source: "/old-path".to_string(),
            destination: "/new-path.html".to_string(),
        }];

        // Test the full chain: rewrite -> clean URLs -> trailing slash
        let path = "/old-path";
        let step1 = apply_url_rewrites(path, &rewrites); // "/new-path.html"
        let step2 = apply_clean_urls(&step1); // "/new-path"
        let step3 = apply_trailing_slash(&step2, true); // "/new-path/"

        assert_eq!(step1, "/new-path.html");
        assert_eq!(step2, "/new-path");
        assert_eq!(step3, "/new-path/");
    }

    #[test]
    fn test_empty_and_malformed_rewrites() {
        let empty_rewrites: Vec<Rewrite> = vec![];
        assert_eq!(
            apply_url_rewrites("/any/path", &empty_rewrites),
            "/any/path"
        );

        let malformed_rewrites = vec![
            Rewrite {
                source: "".to_string(),
                destination: "/fallback".to_string(),
            },
            Rewrite {
                source: "/valid".to_string(),
                destination: "".to_string(),
            },
        ];

        // Empty source doesn't match (new logic requires exact match, wildcard, or regex pattern)
        assert_eq!(
            apply_url_rewrites("/test", &malformed_rewrites),
            "/test"
        );

        // "/valid" exact matches second rule
        assert_eq!(
            apply_url_rewrites("/valid", &malformed_rewrites),
            ""
        );
    }

    #[test]
    fn test_rewrite_exact_vs_prefix_matching() {
        let rewrites = vec![
            Rewrite {
                source: "/api".to_string(),
                destination: "/api-exact".to_string(),
            },
            Rewrite {
                source: "/api/*".to_string(),
                destination: "/api-wildcard/".to_string(),
            },
        ];

        // Exact match for "/api"
        assert_eq!(apply_url_rewrites("/api", &rewrites), "/api-exact");

        // "/api/users" matches the wildcard pattern "/api/*"
        assert_eq!(apply_url_rewrites("/api/users", &rewrites), "/api-wildcard/");

        // Non-matching paths
        assert_eq!(apply_url_rewrites("/apiold", &rewrites), "/apiold"); // No match - not exact and not "/api/*"
        assert_eq!(apply_url_rewrites("/other", &rewrites), "/other");
    }

    #[test]
    fn test_special_characters_in_paths() {
        // Test paths with special characters
        assert!(should_use_spa_fallback("/café/menu"));
        assert!(should_use_spa_fallback("/résumé"));
        assert!(should_use_spa_fallback("/测试/页面"));

        // Test URL encoding scenarios
        assert!(should_use_spa_fallback("/user%20profile"));
        assert!(should_use_spa_fallback("/search%3Fq%3Dtest"));

        // These should still not use SPA fallback due to dot
        assert!(!should_use_spa_fallback("/café.html"));
        assert!(!should_use_spa_fallback("/测试.js"));
    }

    #[test]
    fn test_case_sensitivity() {
        let rewrites = vec![
            Rewrite {
                source: "/API/*".to_string(),
                destination: "/api-upper/".to_string(),
            },
            Rewrite {
                source: "/api/*".to_string(),
                destination: "/api-lower/".to_string(),
            },
        ];

        // Test case sensitivity (current implementation is case-sensitive)
        assert_eq!(apply_url_rewrites("/API/users", &rewrites), "/api-upper/");
        assert_eq!(apply_url_rewrites("/api/users", &rewrites), "/api-lower/");
        assert_eq!(apply_url_rewrites("/Api/users", &rewrites), "/Api/users"); // No match
    }
}
