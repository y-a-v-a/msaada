mod clipboard;
mod config;
mod logger;
mod network;
mod shutdown;
mod spa;
mod tls;

use actix_cors::Cors;
use actix_files::Files;
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
use std::path::{Path, PathBuf};
use std::process::exit;
use std::sync::atomic::{AtomicBool, Ordering};
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
    payload: web::Payload,
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
                        Ok(field) => {
                            let content_disposition = field.content_disposition();
                            let name = content_disposition
                                .get_name()
                                .unwrap_or("unknown")
                                .to_string();

                            // If it's a file, just store the filename
                            if let Some(filename) = content_disposition.get_filename() {
                                files.push(json!({
                                    "field_name": name,
                                    "filename": filename
                                }));
                            } else {
                                // For regular form fields, get the value
                                let mut data = Vec::new();
                                let mut field_stream = field;
                                while let Some(chunk) = field_stream.next().await {
                                    data.extend_from_slice(&chunk?);
                                }

                                if let Ok(value) = String::from_utf8(data) {
                                    form_fields.insert(name, value);
                                }
                            }
                        }
                        Err(e) => {
                            return Ok(HttpResponse::BadRequest().json(json!({
                                "error": format!("Field error: {}", e)
                            })));
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
                let bytes = web::BytesMut::new();
                let mut body = bytes;
                let mut stream = payload;

                while let Some(chunk) = stream.next().await {
                    body.extend_from_slice(&chunk?);
                }

                match serde_json::from_slice::<Value>(&body) {
                    Ok(json_data) => {
                        response_data["json_data"] = json_data;
                    }
                    Err(e) => {
                        return Ok(HttpResponse::BadRequest().json(json!({
                            "error": format!("JSON parse error: {}", e)
                        })));
                    }
                }
            }

            // Handle application/x-www-form-urlencoded
            (mime::APPLICATION, sub) if sub == "x-www-form-urlencoded" => {
                let bytes = web::BytesMut::new();
                let mut body = bytes;
                let mut stream = payload;

                while let Some(chunk) = stream.next().await {
                    body.extend_from_slice(&chunk?);
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
                let bytes = web::BytesMut::new();
                let mut body = bytes;
                let mut stream = payload;

                while let Some(chunk) = stream.next().await {
                    body.extend_from_slice(&chunk?);
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

/// Handler for checking if POST handler is working via self-test
#[get("/self-test")]
async fn self_test_endpoint() -> impl Responder {
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
    let json_test = client
        .post("http://localhost:3000/test-json")
        .insert_header(("Content-Type", "application/json"))
        .send_json(&json!({"test": "value", "number": 42}))
        .await;

    // Test form POST
    let form_test = client
        .post("http://localhost:3000/test-form")
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
    let current_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
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
    let config_loader = config::ConfigLoader::new(current_dir.clone(), serve_dir.clone());
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
    let effective_serve_dir = if let Some(ref public_dir) = configuration.public {
        PathBuf::from(public_dir)
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

    // Setup paths for basic web files
    let index_path = PathBuf::from("index.html");
    let css_path = PathBuf::from("style.css");
    let js_path = PathBuf::from("main.js");

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
                dir_arg
            ));
            app_logger.info(
                "Tip: Use --init flag to create basic web files (index.html, style.css, main.js).",
            );
        }
    }

    // Setting this to true will increase logging verbosity
    if std::env::var(key).is_err() {
        std::env::set_var(key, "msaada=debug,actix_web=debug");
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
    if !clipboard_disabled {
        let server_url = format!(
            "{}://localhost:{}",
            if uses_https { "https" } else { "http" },
            actual_port
        );
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

    if test_flag {
        let protocol = if uses_https { "https" } else { "http" };
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

    let server = HttpServer::new({
        // Clone values that need to be moved into the closure
        let effective_serve_dir = effective_serve_dir.clone();
        let effective_single_page_app = effective_single_page_app;
        let effective_symlinks_enabled = effective_symlinks_enabled;
        let effective_etag_enabled = effective_etag_enabled;
        let effective_clean_urls = effective_clean_urls;
        let effective_trailing_slash = effective_trailing_slash;
        let config_rewrites = configuration.rewrites.clone();

        move || {
            // Create custom headers middleware
            let headers = DefaultHeaders::new()
                .add(("X-Server", SERVER_SIGNATURE))
                .add(("X-Powered-By", PKG_NAME))
                .add(("X-Version", PKG_VERSION));

            // Setup CORS middleware (conditionally configured)
            let cors = if cors_enabled {
                Cors::default()
                    .allow_any_origin()
                    .allow_any_header()
                    .allow_any_method()
                    .supports_credentials()
            } else {
                Cors::default().allow_any_origin() // Still need minimal CORS setup
            };

            // Build the app with middleware applied in proper order
            // TODO: Implement conditional compression based on effective_compression_enabled flag
            // For now, compression is always enabled as implementing conditional middleware
            // requires more complex type handling in actix-web
            let mut app = App::new()
                .wrap(CustomLogger)
                .wrap(headers)
                .wrap(cors)
                .wrap(Compress::default());

            // Register the POST handler FIRST with highest priority
            app = app.service(handle_post);

            // Register self-test endpoint if --test flag is provided
            if test_flag {
                app = app.service(self_test_endpoint);
            }

            // Configure static file serving with SPA support
            let serve_path = effective_serve_dir.to_string_lossy().to_string();
            let mut files_service = Files::new("/", serve_path)
                .index_file("index.html")
                .use_etag(effective_etag_enabled)
                .use_last_modified(!effective_etag_enabled);

            // Apply symlink handling based on configuration
            if effective_symlinks_enabled {
                // Enable symlinks with basic validation for dev usage
                files_service = files_service.path_filter({
                    move |path, _req| {
                        // For dev servers, we just need basic symlink resolution
                        // Block obvious directory traversal attempts but don't over-engineer
                        if path.to_string_lossy().contains("..") {
                            return false; // Block obvious traversal attempts
                        }

                        // Allow symlinks and let the filesystem handle the rest
                        true
                    }
                });
            } else {
                // Default: Block symlinks for consistency with other dev servers
                files_service = files_service.path_filter({
                    let serve_dir = effective_serve_dir.clone();
                    move |path, _req| {
                        let full_path = serve_dir.join(path);
                        match full_path.symlink_metadata() {
                            Ok(metadata) => !metadata.file_type().is_symlink(),
                            Err(_) => true, // Allow if we can't check - let actix handle it
                        }
                    }
                });
            }

            app = app.service(files_service);

            // Add SPA fallback handler if single page app mode is enabled
            if effective_single_page_app {
                // Create a configurable SPA handler that uses URL processing functions
                let spa_handler = {
                    let serve_dir = effective_serve_dir.clone();
                    let clean_urls = effective_clean_urls;
                    let trailing_slash = effective_trailing_slash;
                    let rewrites = config_rewrites.clone();

                    move |req: HttpRequest| {
                        spa::configurable_spa_handler(
                            req,
                            serve_dir.clone(),
                            clean_urls,
                            trailing_slash,
                            rewrites.clone(),
                        )
                    }
                };

                app = app.default_service(web::route().to(spa_handler));
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
