use actix_files::Files;
use actix_web::{middleware::Logger, App, HttpServer};
use clap::Arg;
use clap::Command;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::exit;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Define the template content from external files
    const HTML_TEMPLATE: &str = include_str!("index_template.html");
    const CSS_TEMPLATE: &str = include_str!("style_template.css");
    const JS_TEMPLATE: &str = include_str!("main_template.js");
    
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

    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    log::info!("starting HTTP server at http://localhost:{0}", port_arg);

    HttpServer::new(|| {
        App::new()
            .service(Files::new("/", "./").index_file("index.html"))
            .wrap(Logger::default().log_target("msaada"))
    })
    .bind(("127.0.0.1", port))?
    .run()
    .await
}