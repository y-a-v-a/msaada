use actix_files::Files;
use actix_web::{middleware::Logger, App, HttpServer};
use clap::Arg;
use clap::Command;
use std::env;
use std::path::Path;
use std::process::exit;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
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
