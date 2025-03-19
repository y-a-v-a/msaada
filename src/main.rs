use actix_files::Files;
use actix_web::{
    middleware::{Logger, DefaultHeaders}, 
    web, App, Error, HttpRequest, HttpResponse, HttpServer, 
    Responder, post, get
};
use actix_multipart::Multipart;
use clap::Arg;
use clap::Command;
use futures_util::stream::StreamExt;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::exit;
use std::sync::atomic::{AtomicBool, Ordering};

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
            },
            
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
                    },
                    Err(e) => {
                        return Ok(HttpResponse::BadRequest().json(json!({
                            "error": format!("JSON parse error: {}", e)
                        })));
                    }
                }
            },
            
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
                        let decoded_key = urlencoding::decode(key).unwrap_or_else(|_| key.into()).to_string();
                        let decoded_value = urlencoding::decode(value).unwrap_or_else(|_| value.into()).to_string();
                        
                        form_fields.insert(decoded_key, decoded_value);
                    }
                }
                
                response_data["form_data"] = json!(form_fields);
            },
            
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
                    },
                    Err(_) => {
                        response_data["binary_data"] = json!("<binary data received>");
                    }
                }
            },
            
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
    let json_test = client.post("http://localhost:3000/test-json")
        .insert_header(("Content-Type", "application/json"))
        .send_json(&json!({"test": "value", "number": 42}))
        .await;
        
    // Test form POST
    let form_test = client.post("http://localhost:3000/test-form")
        .insert_header(("Content-Type", "application/x-www-form-urlencoded"))
        .send_body("name=test&value=123")
        .await;
        
    // Check results
    let json_success = match json_test {
        Ok(mut res) => {
            if res.status().is_success() {
                match res.json::<Value>().await {
                    Ok(body) => {
                        body.get("json_data").is_some()
                    },
                    Err(_) => false
                }
            } else {
                false
            }
        },
        Err(_) => false
    };
    
    let form_success = match form_test {
        Ok(mut res) => {
            if res.status().is_success() {
                match res.json::<Value>().await {
                    Ok(body) => {
                        body.get("form_data").is_some()
                    },
                    Err(_) => false
                }
            } else {
                false
            }
        },
        Err(_) => false
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
    
    // Log startup information
    println!("Starting {} v{} by {}", PKG_NAME, PKG_VERSION, PKG_AUTHORS);
    
    let key = "RUST_LOG";
    env::set_var(key, "msaada=info");

    let matches = Command::new("Msaada")
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .required(true)
                .help("The port number to use"),
        )
        .arg(
            Arg::new("directory")
                .short('d')
                .long("dir")
                .required(true)
                .help("The directory to serve from"),
        )
        .arg(
            Arg::new("init")
                .long("init")
                .required(false)
                .action(clap::ArgAction::SetTrue)
                .help("Initialize a basic webpage (index.html, style.css, main.js) in the specified directory"),
        )
        .arg(
            Arg::new("test")
                .long("test")
                .required(false)
                .action(clap::ArgAction::SetTrue)
                .help("Enable self-test endpoint at /self-test to verify POST handler functionality"),
        )
        .get_matches();

    let port_arg = matches.get_one::<String>("port").unwrap();
    let port = port_arg.parse::<u16>().unwrap();

    let dir_arg = matches.get_one::<String>("directory").unwrap();
    let dir = Path::new(&dir_arg);
    let is_path_set = env::set_current_dir(dir);

    match is_path_set {
        Ok(()) => (),
        Err(_) => {
            println!("Unknown path: {}", dir_arg);
            exit(1)
        }
    }

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
            println!("Created files: {}", created_files.join(", "));
        } else {
            println!("All basic web files already exist. No files created.");
        }
    } else {
        // Without --init flag, only check if index.html exists and prompt if missing
        if !index_path.exists() {
            println!("index.html not found in {}.", dir_arg);
            println!("Would you like to create a basic index.html file? (y/n)");
            
            let mut response = String::new();
            io::stdin().read_line(&mut response)?;
            
            if response.trim().to_lowercase() == "y" {
                fs::write(&index_path, HTML_TEMPLATE)?;
                println!("Created index.html file.");
                println!("Tip: Use --init flag to also create style.css and main.js files.");
            } else {
                println!("Note: The server will run but may not serve a default page.");
            }
        }
    }

    // Setting this to true will increase logging verbosity
    if std::env::var(key).is_err() {
        std::env::set_var(key, "msaada=debug,actix_web=debug");
    }
    
    // Initialize logging
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    
    // Server information message
    println!("Server: {}", SERVER_SIGNATURE);
    
    if test_flag {
        log::info!("Self-test endpoint enabled at http://localhost:{}/self-test", port_arg);
    }
    
    log::info!("starting HTTP server at http://localhost:{}", port_arg);
    
    HttpServer::new(move || {
        // Create custom headers middleware
        let headers = DefaultHeaders::new()
            .add(("X-Server", SERVER_SIGNATURE))
            .add(("X-Powered-By", PKG_NAME))
            .add(("X-Version", PKG_VERSION));
            
        // Create a customized logger with higher detail level
        let logger = Logger::new("%r %s %b %D")
            .log_target("msaada");
        
        // Build the app with correct ordering of handlers
        let mut app = App::new()
            // First register middleware
            .wrap(logger)
            .wrap(headers);
            
        // Register the POST handler FIRST with highest priority
        app = app.service(handle_post);
            
        // Register self-test endpoint if --test flag is provided
        if test_flag {
            app = app.service(self_test_endpoint);
        }
        
        // Serve static files with lower priority (after POST handler)
        app = app.service(
            Files::new("/", "./")
                .index_file("index.html")
                .use_last_modified(true)
        );
        
        app
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}