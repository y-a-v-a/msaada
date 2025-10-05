// src/rewrite.rs
// URL rewriting and routing utilities for msaada

use crate::config::Rewrite;
use regex::Regex;

/// Compiled rewrite rule with regex pattern
#[derive(Clone)]
pub struct CompiledRewrite {
    pub pattern: Regex,
    pub destination: String,
    pub original_source: String,
}

/// Convert Vercel-style patterns to proper regex patterns
fn pattern_to_regex(pattern: &str) -> Result<String, String> {
    // Handle exact matches (no wildcards or regex)
    if !pattern.contains('*') && !pattern.contains('(') && !pattern.contains('[') {
        return Ok(format!("^{}$", regex::escape(pattern)));
    }

    // Handle glob patterns with /*
    if let Some(prefix) = pattern.strip_suffix("/*") {
        return Ok(format!("^{}/.*$", regex::escape(prefix)));
    }

    // Handle catch-all pattern
    if pattern == "**" {
        return Ok("^.*$".to_string());
    }

    // Handle regex patterns with capture groups like /api/(.*)
    // These should already be valid regex, just anchor them
    if pattern.contains("(.*)") || pattern.contains("(.*?)") {
        // Remove leading/trailing slashes if present, we'll add our own anchors
        let cleaned = pattern.trim_start_matches('^').trim_end_matches('$');
        return Ok(format!("^{}$", cleaned));
    }

    // Handle glob patterns with ** in the middle or wildcards
    let mut regex_pattern = String::new();
    let mut chars = pattern.chars().peekable();

    regex_pattern.push('^');

    while let Some(ch) = chars.next() {
        match ch {
            '*' => {
                // Check for **
                if chars.peek() == Some(&'*') {
                    chars.next(); // consume second *
                    regex_pattern.push_str(".*");
                } else {
                    regex_pattern.push_str("[^/]*");
                }
            }
            '?' => {
                regex_pattern.push_str("[^/]");
            }
            '.' | '+' | '(' | ')' | '|' | '[' | ']' | '{' | '}' | '^' | '$' | '\\' => {
                // These are already regex chars, keep them
                regex_pattern.push(ch);
            }
            _ => {
                regex_pattern.push(ch);
            }
        }
    }

    regex_pattern.push('$');
    Ok(regex_pattern)
}

/// Compile rewrite rules from configuration
pub fn compile_rewrites(rewrites: &[Rewrite]) -> Result<Vec<CompiledRewrite>, String> {
    let mut compiled = Vec::new();

    for rewrite in rewrites {
        let regex_pattern = pattern_to_regex(&rewrite.source)?;

        match Regex::new(&regex_pattern) {
            Ok(pattern) => {
                compiled.push(CompiledRewrite {
                    pattern,
                    destination: rewrite.destination.clone(),
                    original_source: rewrite.source.clone(),
                });
                log::info!(
                    "Compiled rewrite: {} -> {} (regex: {})",
                    rewrite.source,
                    rewrite.destination,
                    regex_pattern
                );
            }
            Err(e) => {
                return Err(format!(
                    "Invalid rewrite pattern '{}': {}",
                    rewrite.source, e
                ));
            }
        }
    }

    Ok(compiled)
}

/// Match a request path against rewrite rules and return the destination file path
pub fn match_rewrite(path: &str, rewrites: &[CompiledRewrite]) -> Option<String> {
    for rewrite in rewrites {
        if rewrite.pattern.is_match(path) {
            log::debug!(
                "Rewrite matched: {} -> {} (pattern: {})",
                path,
                rewrite.destination,
                rewrite.original_source
            );
            return Some(rewrite.destination.clone());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    /// Resolve the destination path to a file system path
    fn resolve_destination(destination: &str, serve_dir: &Path) -> PathBuf {
        let cleaned = destination.trim_start_matches('/');
        serve_dir.join(cleaned)
    }

    #[test]
    fn test_pattern_to_regex_exact_match() {
        let regex = pattern_to_regex("/old-path").unwrap();
        // regex::escape escapes the dash, which is safe (though unnecessary outside character classes)
        assert_eq!(regex, "^/old\\-path$");

        let re = Regex::new(&regex).unwrap();
        assert!(re.is_match("/old-path"));
        assert!(!re.is_match("/old-path/extra"));
        assert!(!re.is_match("/other-path"));
    }

    #[test]
    fn test_pattern_to_regex_with_capture_group() {
        let regex = pattern_to_regex("/api/(.*)").unwrap();
        assert_eq!(regex, "^/api/(.*)$");

        let re = Regex::new(&regex).unwrap();
        assert!(re.is_match("/api/test"));
        assert!(re.is_match("/api/users/123"));
        assert!(re.is_match("/api/"));
        assert!(!re.is_match("/other/test"));
    }

    #[test]
    fn test_pattern_to_regex_wildcard() {
        let regex = pattern_to_regex("/api/*").unwrap();
        assert_eq!(regex, "^/api/.*$");

        let re = Regex::new(&regex).unwrap();
        assert!(re.is_match("/api/test"));
        assert!(re.is_match("/api/users/123"));
        assert!(!re.is_match("/other/test"));
    }

    #[test]
    fn test_pattern_to_regex_catch_all() {
        let regex = pattern_to_regex("**").unwrap();
        assert_eq!(regex, "^.*$");

        let re = Regex::new(&regex).unwrap();
        assert!(re.is_match("/anything"));
        assert!(re.is_match("/api/test"));
        assert!(re.is_match(""));
    }

    #[test]
    fn test_compile_rewrites() {
        let rewrites = vec![
            Rewrite {
                source: "/api/(.*)".to_string(),
                destination: "/api.html".to_string(),
            },
            Rewrite {
                source: "/old-path".to_string(),
                destination: "/new-path".to_string(),
            },
        ];

        let compiled = compile_rewrites(&rewrites).unwrap();
        assert_eq!(compiled.len(), 2);

        assert!(compiled[0].pattern.is_match("/api/test"));
        assert!(compiled[1].pattern.is_match("/old-path"));
    }

    #[test]
    fn test_match_rewrite() {
        let rewrites = vec![
            Rewrite {
                source: "/api/(.*)".to_string(),
                destination: "/api.html".to_string(),
            },
            Rewrite {
                source: "/old-path".to_string(),
                destination: "/new-path".to_string(),
            },
        ];

        let compiled = compile_rewrites(&rewrites).unwrap();

        assert_eq!(
            match_rewrite("/api/test", &compiled),
            Some("/api.html".to_string())
        );

        assert_eq!(
            match_rewrite("/old-path", &compiled),
            Some("/new-path".to_string())
        );

        assert_eq!(match_rewrite("/unknown", &compiled), None);
    }

    #[test]
    fn test_resolve_destination() {
        let serve_dir = PathBuf::from("/var/www");

        let path = resolve_destination("/api.html", &serve_dir);
        assert_eq!(path, PathBuf::from("/var/www/api.html"));

        let path = resolve_destination("api.html", &serve_dir);
        assert_eq!(path, PathBuf::from("/var/www/api.html"));
    }
}
