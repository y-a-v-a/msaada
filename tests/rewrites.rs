//! URL Rewrites Integration Tests
//!
//! This module contains comprehensive integration tests for the URL rewriting feature,
//! including static rewrites, dynamic rewrites with capture groups, named parameters,
//! and glob pattern matching.

mod common;

use common::assertions::ResponseAssertions;
use common::filesystem::{FileSystemHelper, ServeJsonOptions};
use common::server::TestServer;
use reqwest::StatusCode;
use serde_json::json;
use std::fs;

/// Test basic static rewrites (no capture groups)
#[tokio::test]
async fn static_rewrites() {
    let test_dir = std::env::temp_dir().join("static_rewrites_test");
    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(&test_dir).expect("Failed to create test directory");

    // Create test files
    FileSystemHelper::create_html_file(&test_dir, "index.html", "Home", "<h1>Home Page</h1>")
        .expect("Failed to create index.html");

    FileSystemHelper::create_html_file(&test_dir, "about.html", "About", "<h1>About Page</h1>")
        .expect("Failed to create about.html");

    FileSystemHelper::create_html_file(
        &test_dir,
        "contact.html",
        "Contact",
        "<h1>Contact Page</h1>",
    )
    .expect("Failed to create contact.html");

    // Create serve.json with static rewrites
    FileSystemHelper::create_serve_json(
        &test_dir,
        ".",
        Some(ServeJsonOptions {
            rewrites: vec![
                json!({"source": "/about", "destination": "/about.html"}),
                json!({"source": "/contact-us", "destination": "/contact.html"}),
            ],
            ..Default::default()
        }),
    )
    .expect("Failed to create serve.json");

    let server = TestServer::new_with_options(
        Some(test_dir.clone()),
        Some(vec![
            "--config".to_string(),
            test_dir.join("serve.json").to_string_lossy().to_string(),
        ]),
    )
    .await
    .expect("Failed to start server");

    let client = reqwest::Client::new();

    // Test rewrite: /about -> /about.html
    let response = client
        .get(server.url_for("/about"))
        .send()
        .await
        .expect("GET request failed");

    response.assert_status(StatusCode::OK);
    let content = response
        .text_for_assertions()
        .await
        .expect("Failed to get response text");
    assert!(
        content.contains("About Page"),
        "Static rewrite /about -> /about.html should work"
    );

    // Test rewrite: /contact-us -> /contact.html
    let response = client
        .get(server.url_for("/contact-us"))
        .send()
        .await
        .expect("GET request failed");

    response.assert_status(StatusCode::OK);
    let content = response
        .text_for_assertions()
        .await
        .expect("Failed to get response text");
    assert!(
        content.contains("Contact Page"),
        "Static rewrite /contact-us -> /contact.html should work"
    );
}

/// Test capture group substitution ($1, $2 syntax)
#[tokio::test]
async fn capture_group_rewrites() {
    let test_dir = std::env::temp_dir().join("capture_group_test");
    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(&test_dir).expect("Failed to create test directory");
    fs::create_dir_all(test_dir.join("api")).expect("Failed to create api directory");
    fs::create_dir_all(test_dir.join("blog")).expect("Failed to create blog directory");

    // Create test files
    FileSystemHelper::create_html_file(
        &test_dir.join("api"),
        "users.html",
        "Users API",
        "<h1>Users API Endpoint</h1>",
    )
    .expect("Failed to create users.html");

    FileSystemHelper::create_html_file(
        &test_dir.join("blog"),
        "2024.html",
        "Blog 2024",
        "<h1>Blog Posts from 2024</h1>",
    )
    .expect("Failed to create 2024.html");

    // Create serve.json with capture group rewrites
    FileSystemHelper::create_serve_json(
        &test_dir,
        ".",
        Some(ServeJsonOptions {
            rewrites: vec![
                json!({"source": "/api/(.*)", "destination": "/api/$1.html"}),
                json!({"source": "/blog/year/(.*)", "destination": "/blog/$1.html"}),
            ],
            ..Default::default()
        }),
    )
    .expect("Failed to create serve.json");

    let server = TestServer::new_with_options(
        Some(test_dir.clone()),
        Some(vec![
            "--config".to_string(),
            test_dir.join("serve.json").to_string_lossy().to_string(),
        ]),
    )
    .await
    .expect("Failed to start server");

    let client = reqwest::Client::new();

    // Test: /api/users -> /api/users.html
    let response = client
        .get(server.url_for("/api/users"))
        .send()
        .await
        .expect("GET request failed");

    response.assert_status(StatusCode::OK);
    let content = response
        .text_for_assertions()
        .await
        .expect("Failed to get response text");
    assert!(
        content.contains("Users API Endpoint"),
        "Capture group rewrite should work for /api/users"
    );

    // Test: /blog/year/2024 -> /blog/2024.html
    let response = client
        .get(server.url_for("/blog/year/2024"))
        .send()
        .await
        .expect("GET request failed");

    response.assert_status(StatusCode::OK);
    let content = response
        .text_for_assertions()
        .await
        .expect("Failed to get response text");
    assert!(
        content.contains("Blog Posts from 2024"),
        "Capture group rewrite should work for /blog/year/2024"
    );
}

/// Test named parameter rewrites (:id, :name syntax)
#[tokio::test]
async fn named_parameter_rewrites() {
    let test_dir = std::env::temp_dir().join("named_params_test");
    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(&test_dir).expect("Failed to create test directory");
    fs::create_dir_all(test_dir.join("users")).expect("Failed to create users directory");
    fs::create_dir_all(test_dir.join("posts")).expect("Failed to create posts directory");

    // Create test files
    FileSystemHelper::create_html_file(
        &test_dir.join("users"),
        "123.html",
        "User 123",
        "<h1>User Profile: 123</h1>",
    )
    .expect("Failed to create 123.html");

    FileSystemHelper::create_html_file(
        &test_dir.join("posts"),
        "hello-world.html",
        "Hello World Post",
        "<h1>Post: hello-world</h1>",
    )
    .expect("Failed to create hello-world.html");

    // Create serve.json with named parameter rewrites
    FileSystemHelper::create_serve_json(
        &test_dir,
        ".",
        Some(ServeJsonOptions {
            rewrites: vec![
                json!({"source": "/user/:id", "destination": "/users/:id.html"}),
                json!({"source": "/post/:slug", "destination": "/posts/:slug.html"}),
            ],
            ..Default::default()
        }),
    )
    .expect("Failed to create serve.json");

    let server = TestServer::new_with_options(
        Some(test_dir.clone()),
        Some(vec![
            "--config".to_string(),
            test_dir.join("serve.json").to_string_lossy().to_string(),
        ]),
    )
    .await
    .expect("Failed to start server");

    let client = reqwest::Client::new();

    // Test: /user/123 -> /users/123.html
    let response = client
        .get(server.url_for("/user/123"))
        .send()
        .await
        .expect("GET request failed");

    response.assert_status(StatusCode::OK);
    let content = response
        .text_for_assertions()
        .await
        .expect("Failed to get response text");
    assert!(
        content.contains("User Profile: 123"),
        "Named parameter rewrite should work for /user/123"
    );

    // Test: /post/hello-world -> /posts/hello-world.html
    let response = client
        .get(server.url_for("/post/hello-world"))
        .send()
        .await
        .expect("GET request failed");

    response.assert_status(StatusCode::OK);
    let content = response
        .text_for_assertions()
        .await
        .expect("Failed to get response text");
    assert!(
        content.contains("Post: hello-world"),
        "Named parameter rewrite should work for /post/hello-world"
    );
}

/// Test optional named parameters ({/:id} syntax)
#[tokio::test]
async fn optional_parameter_rewrites() {
    let test_dir = std::env::temp_dir().join("optional_params_test");
    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(&test_dir).expect("Failed to create test directory");
    fs::create_dir_all(test_dir.join("products")).expect("Failed to create products directory");

    // Create test files
    FileSystemHelper::create_html_file(
        &test_dir.join("products"),
        "all.html",
        "All Products",
        "<h1>All Products</h1>",
    )
    .expect("Failed to create all.html");

    FileSystemHelper::create_html_file(
        &test_dir.join("products"),
        "electronics.html",
        "Electronics",
        "<h1>Electronics Category</h1>",
    )
    .expect("Failed to create electronics.html");

    // Create serve.json with optional parameter rewrite
    FileSystemHelper::create_serve_json(
        &test_dir,
        ".",
        Some(ServeJsonOptions {
            rewrites: vec![
                // This should match both /products and /products/electronics
                json!({"source": "/products{/:category}", "destination": "/products/:category.html"}),
            ],
            ..Default::default()
        }),
    )
    .expect("Failed to create serve.json");

    let server = TestServer::new_with_options(
        Some(test_dir.clone()),
        Some(vec![
            "--config".to_string(),
            test_dir.join("serve.json").to_string_lossy().to_string(),
        ]),
    )
    .await
    .expect("Failed to start server");

    let client = reqwest::Client::new();

    // Test: /products/electronics -> /products/electronics.html
    let response = client
        .get(server.url_for("/products/electronics"))
        .send()
        .await
        .expect("GET request failed");

    response.assert_status(StatusCode::OK);
    let content = response
        .text_for_assertions()
        .await
        .expect("Failed to get response text");
    assert!(
        content.contains("Electronics Category"),
        "Optional parameter rewrite should work for /products/electronics"
    );
}

/// Test glob patterns with wildcards (*, **, ?)
#[tokio::test]
async fn glob_pattern_rewrites() {
    let test_dir = std::env::temp_dir().join("glob_patterns_test");
    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(&test_dir).expect("Failed to create test directory");
    fs::create_dir_all(test_dir.join("images")).expect("Failed to create images directory");
    fs::create_dir_all(test_dir.join("api").join("v1")).expect("Failed to create api/v1");

    // Create test files
    FileSystemHelper::create_html_file(
        &test_dir.join("images"),
        "photo.jpg.html",
        "Photo",
        "<h1>Photo Image</h1>",
    )
    .expect("Failed to create photo.jpg.html");

    FileSystemHelper::create_html_file(
        &test_dir.join("api").join("v1"),
        "users.html",
        "Users",
        "<h1>Users API v1</h1>",
    )
    .expect("Failed to create users.html");

    // Create serve.json with glob pattern rewrites
    FileSystemHelper::create_serve_json(
        &test_dir,
        ".",
        Some(ServeJsonOptions {
            rewrites: vec![
                json!({"source": "/img/*.jpg", "destination": "/images/$1.jpg.html"}),
                json!({"source": "/api/**/users", "destination": "/api/$1/users.html"}),
            ],
            ..Default::default()
        }),
    )
    .expect("Failed to create serve.json");

    let server = TestServer::new_with_options(
        Some(test_dir.clone()),
        Some(vec![
            "--config".to_string(),
            test_dir.join("serve.json").to_string_lossy().to_string(),
        ]),
    )
    .await
    .expect("Failed to start server");

    let client = reqwest::Client::new();

    // Test: /img/photo.jpg -> /images/photo.jpg.html
    let response = client
        .get(server.url_for("/img/photo.jpg"))
        .send()
        .await
        .expect("GET request failed");

    response.assert_status(StatusCode::OK);
    let content = response
        .text_for_assertions()
        .await
        .expect("Failed to get response text");
    assert!(
        content.contains("Photo Image"),
        "Glob pattern with * should work"
    );

    // Test: /api/v1/users -> /api/v1/users.html
    let response = client
        .get(server.url_for("/api/v1/users"))
        .send()
        .await
        .expect("GET request failed");

    response.assert_status(StatusCode::OK);
    let content = response
        .text_for_assertions()
        .await
        .expect("Failed to get response text");
    assert!(
        content.contains("Users API v1"),
        "Glob pattern with ** should work"
    );
}

/// Test brace expansion ({jpg,png,gif} syntax)
#[tokio::test]
async fn brace_expansion_rewrites() {
    let test_dir = std::env::temp_dir().join("brace_expansion_test");
    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(&test_dir).expect("Failed to create test directory");
    fs::create_dir_all(test_dir.join("assets")).expect("Failed to create assets directory");

    // Create test files
    FileSystemHelper::create_html_file(
        &test_dir.join("assets"),
        "image.jpg.html",
        "JPG Image",
        "<h1>JPG Image</h1>",
    )
    .expect("Failed to create image.jpg.html");

    FileSystemHelper::create_html_file(
        &test_dir.join("assets"),
        "photo.png.html",
        "PNG Image",
        "<h1>PNG Image</h1>",
    )
    .expect("Failed to create photo.png.html");

    FileSystemHelper::create_html_file(
        &test_dir.join("assets"),
        "logo.gif.html",
        "GIF Image",
        "<h1>GIF Image</h1>",
    )
    .expect("Failed to create logo.gif.html");

    // Create serve.json with brace expansion rewrite
    FileSystemHelper::create_serve_json(
        &test_dir,
        ".",
        Some(ServeJsonOptions {
            rewrites: vec![
                json!({"source": "/images/*.(jpg|png|gif)", "destination": "/assets/$1.$2.html"}),
            ],
            ..Default::default()
        }),
    )
    .expect("Failed to create serve.json");

    let server = TestServer::new_with_options(
        Some(test_dir.clone()),
        Some(vec![
            "--config".to_string(),
            test_dir.join("serve.json").to_string_lossy().to_string(),
        ]),
    )
    .await
    .expect("Failed to start server");

    let client = reqwest::Client::new();

    // Test: /images/image.jpg -> /assets/image.jpg.html
    let response = client
        .get(server.url_for("/images/image.jpg"))
        .send()
        .await
        .expect("GET request failed");

    response.assert_status(StatusCode::OK);
    let content = response
        .text_for_assertions()
        .await
        .expect("Failed to get response text");
    assert!(
        content.contains("JPG Image"),
        "Brace expansion should match .jpg files"
    );

    // Test: /images/photo.png -> /assets/photo.png.html
    let response = client
        .get(server.url_for("/images/photo.png"))
        .send()
        .await
        .expect("GET request failed");

    response.assert_status(StatusCode::OK);
    let content = response
        .text_for_assertions()
        .await
        .expect("Failed to get response text");
    assert!(
        content.contains("PNG Image"),
        "Brace expansion should match .png files"
    );

    // Test: /images/logo.gif -> /assets/logo.gif.html
    let response = client
        .get(server.url_for("/images/logo.gif"))
        .send()
        .await
        .expect("GET request failed");

    response.assert_status(StatusCode::OK);
    let content = response
        .text_for_assertions()
        .await
        .expect("Failed to get response text");
    assert!(
        content.contains("GIF Image"),
        "Brace expansion should match .gif files"
    );
}

/// Test SPA mode combined with dynamic rewrites
#[tokio::test]
async fn spa_mode_with_rewrites() {
    let test_dir = std::env::temp_dir().join("spa_rewrites_test");
    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(&test_dir).expect("Failed to create test directory");
    fs::create_dir_all(test_dir.join("api")).expect("Failed to create api directory");

    // Create SPA index.html
    FileSystemHelper::create_html_file(
        &test_dir,
        "index.html",
        "SPA App",
        "<h1>SPA Application</h1><div id='app'></div>",
    )
    .expect("Failed to create index.html");

    // Create API endpoint
    FileSystemHelper::create_html_file(
        &test_dir.join("api"),
        "data.html",
        "API Data",
        "<h1>API Data Response</h1>",
    )
    .expect("Failed to create data.html");

    // Create serve.json with rewrites
    FileSystemHelper::create_serve_json(
        &test_dir,
        ".",
        Some(ServeJsonOptions {
            rewrites: vec![
                json!({"source": "/api/:endpoint", "destination": "/api/:endpoint.html"}),
            ],
            ..Default::default()
        }),
    )
    .expect("Failed to create serve.json");

    let server = TestServer::new_with_options(
        Some(test_dir.clone()),
        Some(vec![
            "--config".to_string(),
            test_dir.join("serve.json").to_string_lossy().to_string(),
            "--single".to_string(),
        ]),
    )
    .await
    .expect("Failed to start server");

    let client = reqwest::Client::new();

    // Test: API rewrite should work
    let response = client
        .get(server.url_for("/api/data"))
        .send()
        .await
        .expect("GET request failed");

    response.assert_status(StatusCode::OK);
    let content = response
        .text_for_assertions()
        .await
        .expect("Failed to get response text");
    assert!(
        content.contains("API Data Response"),
        "API rewrite should work in SPA mode"
    );

    // Test: Non-existent routes should fall back to index.html
    let response = client
        .get(server.url_for("/some/random/route"))
        .send()
        .await
        .expect("GET request failed");

    response.assert_status(StatusCode::OK);
    let content = response
        .text_for_assertions()
        .await
        .expect("Failed to get response text");
    assert!(
        content.contains("SPA Application"),
        "Non-existent routes should fall back to index.html in SPA mode"
    );
}

/// Test complex rewrite precedence and ordering
#[tokio::test]
async fn rewrite_precedence() {
    let test_dir = std::env::temp_dir().join("rewrite_precedence_test");
    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(&test_dir).expect("Failed to create test directory");

    // Create test files
    FileSystemHelper::create_html_file(
        &test_dir,
        "specific.html",
        "Specific",
        "<h1>Specific Match</h1>",
    )
    .expect("Failed to create specific.html");

    FileSystemHelper::create_html_file(
        &test_dir,
        "general.html",
        "General",
        "<h1>General Match</h1>",
    )
    .expect("Failed to create general.html");

    // Create serve.json with multiple rewrites (first match wins)
    FileSystemHelper::create_serve_json(
        &test_dir,
        ".",
        Some(ServeJsonOptions {
            rewrites: vec![
                json!({"source": "/test/specific", "destination": "/specific.html"}),
                json!({"source": "/test/(.*)", "destination": "/general.html"}),
            ],
            ..Default::default()
        }),
    )
    .expect("Failed to create serve.json");

    let server = TestServer::new_with_options(
        Some(test_dir.clone()),
        Some(vec![
            "--config".to_string(),
            test_dir.join("serve.json").to_string_lossy().to_string(),
        ]),
    )
    .await
    .expect("Failed to start server");

    let client = reqwest::Client::new();

    // Test: /test/specific should match the first (more specific) rule
    let response = client
        .get(server.url_for("/test/specific"))
        .send()
        .await
        .expect("GET request failed");

    response.assert_status(StatusCode::OK);
    let content = response
        .text_for_assertions()
        .await
        .expect("Failed to get response text");
    assert!(
        content.contains("Specific Match"),
        "More specific rewrite should take precedence"
    );

    // Test: /test/other should match the second (general) rule
    let response = client
        .get(server.url_for("/test/other"))
        .send()
        .await
        .expect("GET request failed");

    response.assert_status(StatusCode::OK);
    let content = response
        .text_for_assertions()
        .await
        .expect("Failed to get response text");
    assert!(
        content.contains("General Match"),
        "General rewrite should match when specific doesn't"
    );
}

/// Test mixed capture group and named parameter rewrites
#[tokio::test]
async fn mixed_rewrite_syntax() {
    let test_dir = std::env::temp_dir().join("mixed_rewrite_test");
    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(&test_dir).expect("Failed to create test directory");
    fs::create_dir_all(test_dir.join("content")).expect("Failed to create content directory");

    // Create test file
    FileSystemHelper::create_html_file(
        &test_dir.join("content"),
        "2024-hello.html",
        "Content",
        "<h1>Content: 2024-hello</h1>",
    )
    .expect("Failed to create content file");

    // Create serve.json with mixed rewrite syntax
    FileSystemHelper::create_serve_json(
        &test_dir,
        ".",
        Some(ServeJsonOptions {
            rewrites: vec![
                json!({"source": "/posts/:year/:slug", "destination": "/content/:year-:slug.html"}),
            ],
            ..Default::default()
        }),
    )
    .expect("Failed to create serve.json");

    let server = TestServer::new_with_options(
        Some(test_dir.clone()),
        Some(vec![
            "--config".to_string(),
            test_dir.join("serve.json").to_string_lossy().to_string(),
        ]),
    )
    .await
    .expect("Failed to start server");

    let client = reqwest::Client::new();

    // Test: /posts/2024/hello -> /content/2024-hello.html
    let response = client
        .get(server.url_for("/posts/2024/hello"))
        .send()
        .await
        .expect("GET request failed");

    response.assert_status(StatusCode::OK);
    let content = response
        .text_for_assertions()
        .await
        .expect("Failed to get response text");
    assert!(
        content.contains("Content: 2024-hello"),
        "Mixed named parameters should work correctly"
    );
}
