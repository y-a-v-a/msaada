mod clipboard;
mod config;
mod logger;
mod network;
mod rewrite;
mod shutdown;
mod spa;
mod tls;

use actix_cors::Cors;
use actix_multipart::Multipart;
use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    get,
    middleware::{Compress, DefaultHeaders},
    post, web, App, Error, HttpRequest, HttpResponse, HttpServer, Responder,
};
use clap::Arg;
use clap::Command;
use futures_util::{future::LocalBoxFuture, stream::StreamExt};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::process::exit;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

// Custom HTTP request logging middleware
pub struct CustomLogger;

impl<S, B> Transform<S, ServiceRequest> for CustomLogger
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = CustomLoggerMiddleware<S>;
    type InitError = ();
    type Future = std::future::Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std::future::ready(Ok(CustomLoggerMiddleware { service }))
    }
}

pub struct CustomLoggerMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for CustomLoggerMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let start_time = Instant::now();

        // Extract request information
        let method = req.method().to_string();
        let path = req.path().to_string();
        let client_ip = req
            .connection_info()
            .realip_remote_addr()
            .unwrap_or("unknown")
            .to_string();

        let fut = self.service.call(req);

        Box::pin(async move {
            let res = fut.await?;
            let response_time = start_time.elapsed().as_millis();
            let status = res.status().as_u16();

            // Log the request using our custom logger
            let logger = logger::get_logger();
            logger.http(
                &client_ip,
                &method,
                &path,
                Some(status),
                Some(response_time),
            );

            Ok(res)
        })
    }
}

/// Handler for POST requests - catches all POST requests
#[post("/{path:.*}")]
async fn handle_post(
    path: web::Path<String>,
    mut payload: web::Payload,
    headers: HttpRequest,
) -> Result<impl Responder, Error> {
    let path_str = path.into_inner();
    log::info!("Received POST request to path: {}", path_str);

    // Extract content type
    let content_type = headers
        .headers()
        .get(actix_web::http::header::CONTENT_TYPE)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("application/octet-stream");

    // Create a response object that will be filled with the request data
    let mut response_data: Value = json!({
        "path": path_str,
        "content_type": content_type,
    });

    // Parse the content type
    if let Ok(mime_type) = content_type.parse::<mime::Mime>() {
        match (mime_type.type_(), mime_type.subtype()) {
            // Handle multipart/form-data (file uploads)
            (mime::MULTIPART, mime::FORM_DATA) => {
                let mut multipart = Multipart::new(headers.headers(), payload);
                let mut files = Vec::new();
                let mut form_fields = HashMap::new();

                while let Some(item) = multipart.next().await {
                    match item {
                        Ok(mut field) => {
                            let content_disposition = field.content_disposition();
                            let name = content_disposition
                                .get_name()
                                .unwrap_or("unknown")
                                .to_string();
                            let filename = content_disposition.get_filename().map(String::from);

                            // If it's a file, just store the filename
                            if let Some(fname) = filename {
                                // Read file data but don't store it (just acknowledge receipt)
                                let mut file_size = 0;
                                while let Some(chunk) = field.next().await {
                                    match chunk {
                                        Ok(data) => file_size += data.len(),
                                        Err(e) => {
                                            log::warn!("Error reading file chunk: {}", e);
                                            break;
                                        }
                                    }
                                }
                                files.push(json!({
                                    "field_name": name,
                                    "filename": fname,
                                    "size": file_size
                                }));
                            } else {
                                // For regular form fields, get the value
                                let mut data = Vec::new();
                                while let Some(chunk) = field.next().await {
                                    match chunk {
                                        Ok(bytes) => data.extend_from_slice(&bytes),
                                        Err(e) => {
                                            log::warn!("Error reading field chunk: {}", e);
                                            break;
                                        }
                                    }
                                }

                                if let Ok(value) = String::from_utf8(data) {
                                    form_fields.insert(name, value);
                                }
                            }
                        }
                        Err(e) => {
                            log::error!("Multipart field error: {}", e);
                            // Don't fail the whole request, just skip this field
                            continue;
                        }
                    }
                }

                response_data["form_data"] = json!(form_fields);
                if !files.is_empty() {
                    response_data["files"] = json!(files);
                }
            }

            // Handle application/json
            (mime::APPLICATION, mime::JSON) => {
                let mut body = web::BytesMut::new();

                while let Some(chunk) = payload.next().await {
                    match chunk {
                        Ok(bytes) => body.extend_from_slice(&bytes),
                        Err(e) => {
                            log::error!("Error reading JSON payload: {}", e);
                            return Ok(HttpResponse::BadRequest().json(json!({
                                "error": format!("Failed to read request body: {}", e)
                            })));
                        }
                    }
                }

                match serde_json::from_slice::<Value>(&body) {
                    Ok(json_data) => {
                        response_data["json_data"] = json_data;
                    }
                    Err(e) => {
                        log::error!("JSON parse error: {}", e);
                        return Ok(HttpResponse::BadRequest().json(json!({
                            "error": format!("JSON parse error: {}", e)
                        })));
                    }
                }
            }

            // Handle application/x-www-form-urlencoded
            (mime::APPLICATION, sub) if sub == "x-www-form-urlencoded" => {
                let mut body = web::BytesMut::new();

                while let Some(chunk) = payload.next().await {
                    match chunk {
                        Ok(bytes) => body.extend_from_slice(&bytes),
                        Err(e) => {
                            log::error!("Error reading form payload: {}", e);
                            return Ok(HttpResponse::BadRequest().json(json!({
                                "error": format!("Failed to read request body: {}", e)
                            })));
                        }
                    }
                }

                let body_str = String::from_utf8_lossy(&body);
                let mut form_fields = HashMap::new();

                for pair in body_str.split('&') {
                    if let Some(index) = pair.find('=') {
                        let key = &pair[..index];
                        let value = &pair[index + 1..];

                        // Simple URL decoding
                        let decoded_key = urlencoding::decode(key)
                            .unwrap_or_else(|_| key.into())
                            .to_string();
                        let decoded_value = urlencoding::decode(value)
                            .unwrap_or_else(|_| value.into())
                            .to_string();

                        form_fields.insert(decoded_key, decoded_value);
                    }
                }

                response_data["form_data"] = json!(form_fields);
            }

            // Handle text/* content type (plain text)
            (mime::TEXT, _) => {
                let mut body = web::BytesMut::new();

                while let Some(chunk) = payload.next().await {
                    match chunk {
                        Ok(bytes) => body.extend_from_slice(&bytes),
                        Err(e) => {
                            log::error!("Error reading text payload: {}", e);
                            return Ok(HttpResponse::BadRequest().json(json!({
                                "error": format!("Failed to read request body: {}", e)
                            })));
                        }
                    }
                }

                match String::from_utf8(body.to_vec()) {
                    Ok(text) => {
                        response_data["text_data"] = json!(text);
                    }
                    Err(_) => {
                        response_data["binary_data"] = json!("<binary data received>");
                    }
                }
            }

            // Handle all other content types as binary
            _ => {
                response_data["binary_data"] = json!("<binary data received>");
            }
        }
    } else {
        // Fallback if content type can't be parsed
        response_data["error"] = json!(format!("Invalid content type: {}", content_type));
    }

    Ok(HttpResponse::Ok().json(response_data))
}

/// Static flag to track if self-test has been run
static SELF_TEST_RUN: AtomicBool = AtomicBool::new(false);

#[derive(Clone)]
struct SelfTestConfig {
    base_url: String,
}

/// Handler for checking if POST handler is working via self-test
#[get("/self-test")]
async fn self_test_endpoint(config: web::Data<SelfTestConfig>) -> impl Responder {
    // Check if test has been run before
    if SELF_TEST_RUN.load(Ordering::SeqCst) {
        return HttpResponse::Ok().json(json!({
            "status": "Test already run",
            "success": true,
            "note": "Server restart required to run test again"
        }));
    }

    // Mark test as run
    SELF_TEST_RUN.store(true, Ordering::SeqCst);

    // Test JSON POST
    let client = awc::Client::new();
    let base_url = &config.base_url;
    let json_test = client
        .post(format!("{}/test-json", base_url))
        .insert_header(("Content-Type", "application/json"))
        .send_json(&json!({"test": "value", "number": 42}))
        .await;

    // Test form POST
    let form_test = client
        .post(format!("{}/test-form", base_url))
        .insert_header(("Content-Type", "application/x-www-form-urlencoded"))
        .send_body("name=test&value=123")
        .await;

    // Check results
    let json_success = match json_test {
        Ok(mut res) => {
            if res.status().is_success() {
                match res.json::<Value>().await {
                    Ok(body) => body.get("json_data").is_some(),
                    Err(_) => false,
                }
            } else {
                false
            }
        }
        Err(_) => false,
    };

    let form_success = match form_test {
        Ok(mut res) => {
            if res.status().is_success() {
                match res.json::<Value>().await {
                    Ok(body) => body.get("form_data").is_some(),
                    Err(_) => false,
                }
            } else {
                false
            }
        }
        Err(_) => false,
    };

    // Return test results
    HttpResponse::Ok().json(json!({
        "status": "Self-test complete",
        "success": json_success && form_success,
        "tests": {
            "json_post": json_success,
            "form_post": form_success
        }
    }))
}

// Define package constants
const PKG_NAME: &str = env!("CARGO_PKG_NAME");
const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const PKG_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
const SERVER_SIGNATURE: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

/// Custom file handler that supports URL rewrites and clean URLs
/// This replaces the standard Files service to enable proper rewrite support
async fn serve_file_with_rewrites(
    req: HttpRequest,
    serve_dir: PathBuf,
    rewrites: Option<Arc<[rewrite::CompiledRewrite]>>,
    clean_urls: bool,
    use_etag: bool,
    symlinks_enabled: bool,
) -> Result<actix_files::NamedFile, Error> {
    let mut path = req.path().to_string();

    // Apply URL rewrites if configured
    if let Some(ref rewrite_rules) = rewrites {
        if let Some(destination) = rewrite::match_rewrite(&path, rewrite_rules.as_ref()) {
            log::debug!("Rewrite matched: {} -> {}", path, destination);
            path = destination;
        }
    }

    let canonical_root = serve_dir
        .canonicalize()
        .unwrap_or_else(|e| {
            log::warn!("Failed to canonicalize serve_dir {:?}: {}", serve_dir, e);
            serve_dir.clone()
        });

    log::debug!("Serve dir: {:?}, canonical: {:?}", serve_dir, canonical_root);

    let sanitized_path = match normalize_request_path(path.trim_start_matches('/')) {
        Some(p) => p,
        None => return Err(actix_web::error::ErrorForbidden("Invalid path")),
    };

    let file_path = serve_dir.join(&sanitized_path);
    log::debug!("Trying to serve file: {:?}", file_path);

    let try_open = |candidate: &Path| -> Result<actix_files::NamedFile, io::Error> {
        if !symlinks_enabled {
            if let Ok(metadata) = candidate.symlink_metadata() {
                if metadata.file_type().is_symlink() {
                    return Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "Symlinks are not allowed",
                    ));
                }
            }
        }

        let mut file = actix_files::NamedFile::open(candidate)?;

        if !symlinks_enabled {
            if let Ok(resolved) = file.path().canonicalize() {
                if !resolved.starts_with(&canonical_root) {
                    return Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "Path escapes serve directory",
                    ));
                }
            }
        }

        if use_etag {
            file = file.use_etag(true).use_last_modified(false);
        } else {
            file = file.use_etag(false).use_last_modified(true);
        }

        Ok(file)
    };

    match try_open(&file_path) {
        Ok(file) => Ok(file),
        Err(err) => {
            if err.kind() == io::ErrorKind::PermissionDenied {
                return Err(actix_web::error::ErrorForbidden(err));
            }

            if clean_urls && !path.ends_with(".html") && !path.ends_with('/') {
                let html_path = file_path.with_extension("html");
                match try_open(&html_path) {
                    Ok(file) => {
                        log::debug!("Clean URL: served {} as {}", path, html_path.display());
                        Ok(file)
                    }
                    Err(html_err) => {
                        if html_err.kind() == io::ErrorKind::PermissionDenied {
                            return Err(actix_web::error::ErrorForbidden(html_err));
                        }

                        let index_path = file_path.join("index.html");
                        match try_open(&index_path) {
                            Ok(file) => Ok(file),
                            Err(index_err) => {
                                if index_err.kind() == io::ErrorKind::PermissionDenied {
                                    Err(actix_web::error::ErrorForbidden(index_err))
                                } else {
                                    Err(actix_web::error::ErrorNotFound(index_err))
                                }
                            }
                        }
                    }
                }
            } else {
                let index_path = file_path.join("index.html");
                match try_open(&index_path) {
                    Ok(file) => Ok(file),
                    Err(index_err) => {
                        if index_err.kind() == io::ErrorKind::PermissionDenied {
                            Err(actix_web::error::ErrorForbidden(index_err))
                        } else {
                            Err(actix_web::error::ErrorNotFound(index_err))
                        }
                    }
                }
            }
        }
    }
}

fn normalize_request_path(path: &str) -> Option<PathBuf> {
    let mut normalized = PathBuf::new();

    for component in Path::new(path).components() {
        match component {
            Component::Prefix(_) => return None,
            Component::RootDir | Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    return None;
                }
            }
            Component::Normal(segment) => normalized.push(segment),
        }
    }

    Some(normalized)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Define the template content from external files
    const HTML_TEMPLATE: &str = include_str!("index_template.html");
    const CSS_TEMPLATE: &str = include_str!("style_template.css");
    const JS_TEMPLATE: &str = include_str!("main_template.js");

    let key = "RUST_LOG";
    env::set_var(key, "msaada=info");

    let matches = Command::new("Msaada")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Vincent Bruijn <vebruijn@gmail.com>")
        .about("A powerful HTTP server for local web development - serve static files with advanced features")
        .long_about("Msaada ('service' in Swahili) is a lightweight yet feature-rich HTTP server designed for local web development.\n\nFeatures include HTTPS support, SPA routing, CORS, compression, automatic port switching, and more.\n\nFor detailed documentation, visit: https://github.com/y-a-v-a/msaada")
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .required(true)
                .help("Port number to serve on (e.g., 3000, 8080)"),
        )
        .arg(
            Arg::new("directory")
                .short('d')
                .long("dir")
                .required(true)
                .help("Directory to serve static files from (defaults to current directory)"),
        )
        .arg(
            Arg::new("init")
                .long("init")
                .required(false)
                .action(clap::ArgAction::SetTrue)
                .help("Create starter web files (index.html, style.css, main.js) in the directory"),
        )
        .arg(
            Arg::new("test")
                .long("test")
                .required(false)
                .action(clap::ArgAction::SetTrue)
                .help("Enable self-test endpoint at /self-test for POST request testing"),
        )
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Path to configuration file (serve.json, now.json, or package.json)"),
        )
        .arg(
            Arg::new("no-request-logging")
                .short('L')
                .long("no-request-logging")
                .action(clap::ArgAction::SetTrue)
                .help("Disable HTTP request logging to keep console output clean"),
        )
        .arg(
            Arg::new("no-timestamps")
                .short('T')
                .long("no-timestamps")
                .action(clap::ArgAction::SetTrue)
                .help("Disable timestamps in log messages"),
        )
        .arg(
            Arg::new("cors")
                .short('C')
                .long("cors")
                .action(clap::ArgAction::SetTrue)
                .help("Enable CORS headers for cross-origin requests (sets Access-Control-Allow-Origin: *)"),
        )
        .arg(
            Arg::new("no-compression")
                .short('u')
                .long("no-compression")
                .action(clap::ArgAction::SetTrue)
                .help("Disable gzip compression (compression is enabled by default)"),
        )
        .arg(
            Arg::new("single")
                .short('s')
                .long("single")
                .action(clap::ArgAction::SetTrue)
                .help("Enable Single Page Application mode - serve index.html for all routes"),
        )
        .arg(
            Arg::new("ssl-cert")
                .long("ssl-cert")
                .value_name("FILE")
                .help("Path to SSL/TLS certificate file (PEM or PKCS12/PFX format)"),
        )
        .arg(
            Arg::new("ssl-key")
                .long("ssl-key")
                .value_name("FILE")
                .help("Path to private key file (required for PEM certificates)"),
        )
        .arg(
            Arg::new("ssl-pass")
                .long("ssl-pass")
                .value_name("FILE")
                .help("Path to file containing certificate passphrase"),
        )
        .arg(
            Arg::new("no-clipboard")
                .short('n')
                .long("no-clipboard")
                .action(clap::ArgAction::SetTrue)
                .help("Don't automatically copy server URL to clipboard"),
        )
        .arg(
            Arg::new("no-port-switching")
                .long("no-port-switching")
                .action(clap::ArgAction::SetTrue)
                .help("Fail if specified port is unavailable (don't auto-switch ports)"),
        )
        .arg(
            Arg::new("symlinks")
                .short('S')
                .long("symlinks")
                .action(clap::ArgAction::SetTrue)
                .help("Follow symbolic links when serving files"),
        )
        .arg(
            Arg::new("no-etag")
                .long("no-etag")
                .action(clap::ArgAction::SetTrue)
                .help("Use Last-Modified header instead of ETag for HTTP caching"),
        )
        .get_matches();

    // Initialize the logger
    let enable_request_logging = !matches.get_flag("no-request-logging");
    let enable_timestamps = !matches.get_flag("no-timestamps");
    logger::init_logger(enable_request_logging, enable_timestamps);
    let app_logger = logger::get_logger();

    // Log startup information using new logger
    app_logger.startup_info(PKG_NAME, PKG_VERSION, PKG_AUTHORS);

    let port_arg = matches.get_one::<String>("port").unwrap();
    let requested_port = port_arg.parse::<u16>().unwrap();

    let dir_arg = matches.get_one::<String>("directory").unwrap();
    let dir = Path::new(&dir_arg);
    let serve_dir = dir.to_path_buf();

    let is_path_set = env::set_current_dir(dir);

    match is_path_set {
        Ok(()) => (),
        Err(_) => {
            app_logger.error(&format!("Unknown path: {}", dir_arg));
            exit(1)
        }
    }

    // Load configuration
    let custom_config = matches.get_one::<String>("config").map(|s| s.as_str());
    let config_loader = config::ConfigLoader::new(serve_dir.clone());
    let configuration = match config_loader.load_configuration(custom_config) {
        Ok(config) => config,
        Err(e) => {
            app_logger.error(&format!("Configuration error: {}", e));
            exit(1);
        }
    };

    // Extract CLI flags
    let cors_enabled = matches.get_flag("cors");
    let compression_disabled = matches.get_flag("no-compression");
    let single_page_app = matches.get_flag("single");
    let ssl_cert = matches.get_one::<String>("ssl-cert").map(|s| s.as_str());
    let ssl_key = matches.get_one::<String>("ssl-key").map(|s| s.as_str());
    let ssl_pass = matches.get_one::<String>("ssl-pass").map(|s| s.as_str());
    let clipboard_disabled = matches.get_flag("no-clipboard");
    let port_switching_disabled = matches.get_flag("no-port-switching");
    let symlinks_enabled = matches.get_flag("symlinks");
    let etag_disabled = matches.get_flag("no-etag");

    // Merge CLI flags with configuration (CLI flags override config)
    let effective_single_page_app = single_page_app || configuration.render_single;
    let effective_symlinks_enabled = symlinks_enabled || configuration.symlinks;
    let effective_etag_enabled = !etag_disabled && configuration.etag;
    let effective_clean_urls = configuration.clean_urls;
    let effective_trailing_slash = configuration.trailing_slash;
    let effective_compression_enabled = !compression_disabled;

    // Use the public directory from configuration if available, otherwise use serve_dir
    // The config loader should have already resolved paths correctly
    let effective_serve_dir = if let Some(ref public_dir) = configuration.public {
        let public_path = PathBuf::from(public_dir);
        app_logger.info(&format!(
            "Using public directory from config: {}",
            public_path.display()
        ));
        public_path
    } else {
        serve_dir.clone()
    };

    // Setup clipboard manager
    let clipboard_manager = clipboard::ClipboardManager::new(!clipboard_disabled);

    // Validate and configure TLS if SSL arguments are provided
    let tls_config = match tls::validate_ssl_args(ssl_cert, ssl_key, ssl_pass) {
        Ok(config) => config,
        Err(e) => {
            app_logger.error(&format!("SSL configuration error: {}", e));
            exit(1);
        }
    };

    // Resolve the port - check availability and auto-switch if needed
    let allow_port_switching = !port_switching_disabled;
    let actual_port = match network::NetworkUtils::resolve_port(
        "0.0.0.0",
        requested_port,
        allow_port_switching,
    ) {
        Ok(port) => port,
        Err(e) => {
            app_logger.error(&e);
            exit(1);
        }
    };

    let previous_port = if actual_port != requested_port {
        Some(requested_port)
    } else {
        None
    };

    // Setup paths for basic web files (relative to effective_serve_dir)
    let index_path = effective_serve_dir.join("index.html");
    let css_path = effective_serve_dir.join("style.css");
    let js_path = effective_serve_dir.join("main.js");

    // Check if --init flag was provided
    let init_flag = matches.get_flag("init");

    // Check if --test flag was provided
    let test_flag = matches.get_flag("test");

    if init_flag {
        // Initialize the files without prompting if --init flag was used
        let index_exists = index_path.exists();
        let css_exists = css_path.exists();
        let js_exists = js_path.exists();

        let mut created_files = Vec::new();

        // Create any missing files from the templates
        if !index_exists {
            fs::write(&index_path, HTML_TEMPLATE)?;
            created_files.push("index.html");
        }

        if !css_exists {
            fs::write(&css_path, CSS_TEMPLATE)?;
            created_files.push("style.css");
        }

        if !js_exists {
            fs::write(&js_path, JS_TEMPLATE)?;
            created_files.push("main.js");
        }

        if !created_files.is_empty() {
            app_logger.info(&format!("Created files: {}", created_files.join(", ")));
        } else {
            app_logger.info("All basic web files already exist. No files created.");
        }
    } else {
        // Without --init flag, only check if index.html exists and warn if missing
        if !index_path.exists() {
            app_logger.warn(&format!(
                "index.html not found in {}. The server will run but may not serve a default page.",
                effective_serve_dir.display()
            ));
            app_logger.info(
                "Tip: Use --init flag to create basic web files (index.html, style.css, main.js).",
            );
        }
    }

    // Initialize logging
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    // Create server addresses for display
    let uses_https = tls_config.is_some();
    let server_addresses = network::NetworkUtils::create_server_addresses(
        "0.0.0.0",
        actual_port,
        uses_https,
        previous_port,
    );

    // Server information message using new logger
    app_logger.server_info(
        SERVER_SIGNATURE,
        &server_addresses.local,
        server_addresses.network.as_deref(),
    );

    // Copy URL to clipboard if enabled
    let protocol = if uses_https { "https" } else { "http" };

    if !clipboard_disabled {
        let server_url = format!("{}://localhost:{}", protocol, actual_port);
        if let Err(e) = clipboard_manager.copy_server_url(&server_url) {
            app_logger.warn(&format!("Could not copy to clipboard: {}", e));
        }
    }

    if let Some(prev_port) = server_addresses.previous_port {
        app_logger.warn(&format!(
            "Port {} was already in use, switched to port {}",
            prev_port, actual_port
        ));
    }

    let self_test_data = if test_flag {
        Some(web::Data::new(SelfTestConfig {
            base_url: format!("{}://localhost:{}", protocol, actual_port),
        }))
    } else {
        None
    };

    if test_flag {
        app_logger.info(&format!(
            "Self-test endpoint enabled at {}://localhost:{}/self-test",
            protocol, actual_port
        ));
    }

    // Log compression settings
    if effective_compression_enabled {
        app_logger.info("Compression: enabled");
    } else {
        app_logger.info("Compression: disabled (--no-compression flag)");
    }

    // Set up graceful shutdown handling
    // Create shutdown manager for future advanced shutdown handling
    let mut _shutdown_manager = shutdown::ShutdownManager::new();

    // For now, continue using basic signal handling as integrating full ShutdownManager
    // requires server handle management which is more complex
    if let Err(e) = shutdown::setup_basic_signal_handling().await {
        app_logger.error(&format!("Failed to setup signal handling: {}", e));
    }

    // Load TLS configuration if provided
    let rustls_config = if let Some(ref tls_cfg) = tls_config {
        match tls_cfg.load_server_config().await {
            Ok(config) => Some(config),
            Err(e) => {
                app_logger.error(&format!("Failed to load TLS configuration: {}", e));
                exit(1);
            }
        }
    } else {
        None
    };

    // Compile rewrites with regex patterns
    let compiled_rewrites: Option<Arc<[rewrite::CompiledRewrite]>> =
        if !configuration.rewrites.is_empty() {
            match rewrite::compile_rewrites(&configuration.rewrites) {
                Ok(compiled) => {
                    log::info!("Loaded {} URL rewrites from configuration", compiled.len());
                    Some(Arc::from(compiled))
                }
                Err(e) => {
                    app_logger.error(&format!("Failed to compile rewrite patterns: {}", e));
                    exit(1);
                }
            }
        } else {
            None
        };

    let server = HttpServer::new({
        // Clone values that need to be moved into the closure
        let effective_serve_dir = effective_serve_dir.clone();
        let rewrites = compiled_rewrites.clone();
        let self_test_data = self_test_data.clone();

        move || {
            // Create custom headers middleware
            let headers = DefaultHeaders::new()
                .add(("Server", PKG_NAME))
                .add(("X-Server", SERVER_SIGNATURE))
                .add(("X-Version", PKG_VERSION));

            // Build the app with middleware applied in proper order
            // Add CORS middleware only if --cors flag is used
            // This matches Vercel's serve behavior: no CORS headers without the flag
            // Same-origin requests work naturally without CORS middleware
            let mut app = App::new()
                .wrap(CustomLogger)
                .wrap(headers)
                .wrap(actix_web::middleware::Condition::new(
                    cors_enabled,
                    Cors::permissive(),
                ))
                .wrap(actix_web::middleware::Condition::new(
                    effective_compression_enabled,
                    Compress::default(),
                ))
                // Register the POST handler FIRST with highest priority
                .service(handle_post);

            // Register self-test endpoint if --test flag is provided
            if let Some(ref data) = self_test_data {
                app = app.app_data(data.clone());
                app = app.service(self_test_endpoint);
            }

            // Register custom file handler with rewrite and clean URLs support
            // This replaces the standard Files service to enable proper URL rewriting
            let file_handler = {
                let serve_dir = effective_serve_dir.clone();
                let rewrites_clone = rewrites.clone();
                let clean_urls = effective_clean_urls;
                let use_etag = effective_etag_enabled;
                let symlinks = effective_symlinks_enabled;

                move |req: HttpRequest| {
                    serve_file_with_rewrites(
                        req,
                        serve_dir.clone(),
                        rewrites_clone.clone(),
                        clean_urls,
                        use_etag,
                        symlinks,
                    )
                }
            };

            // Note: Directory listing feature removed in favor of simpler file serving
            // This is acceptable for a development server

            // Add SPA fallback handler if single page app mode is enabled
            // Otherwise use the file handler as default service
            if effective_single_page_app {
                // Create a configurable SPA handler that uses URL processing functions
                let spa_handler = {
                    let serve_dir = effective_serve_dir.clone();
                    let clean_urls = effective_clean_urls;
                    let trailing_slash = effective_trailing_slash;

                    move |req: HttpRequest| {
                        spa::simple_spa_handler(req, serve_dir.clone(), clean_urls, trailing_slash)
                    }
                };

                app = app.default_service(web::route().to(spa_handler));
            } else {
                // Use file handler as default service (catches all unmatched routes)
                app = app.default_service(web::get().to(file_handler));
            }

            app
        }
    });

    // Bind server with or without TLS
    let server = if let Some(rustls_config) = rustls_config {
        server.bind_rustls_021(("0.0.0.0", actual_port), rustls_config)?
    } else {
        server.bind(("0.0.0.0", actual_port))?
    };

    // Start the server
    server.run().await
}
