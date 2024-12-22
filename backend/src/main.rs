mod api;
mod cfg;
mod ds;
mod dto;
mod env;
mod err;
mod event;
mod log;
mod sdcard;
mod steam;
mod watch;
mod ws;

use crate::cfg::CONFIG;
use crate::ds::Store;
use crate::env::*;
use crate::watch::start_watch;
use crate::{api::config, dto::CardEvent};
use actix_cors::Cors;
use actix_web::{web, App, HttpServer};
use err::Error;
use futures::{pin_mut, select, FutureExt};
use log::create_subscriber;
use std::path::PathBuf;
use std::process::exit;
use std::sync::Arc;
use tokio::sync::broadcast::{self, Sender};
use tracing::{debug, error, info};

pub async fn init() {
	create_subscriber().await;
}

type MainResult = Result<(), Error>;

async fn run_web_server(port: u16, datastore: Arc<Store>, sender: Sender<CardEvent>) -> MainResult {
	info!("Starting HTTP server on port {port}...");

	HttpServer::new(move || {
		let cors = Cors::default()
			.allow_any_header()
			.allow_any_method()
			.allow_any_origin()
			.max_age(3600);

		App::new()
			.wrap(cors)
			// .app_data(web::Data::new(api::AppState{datastore: datastore.clone()}))
			.app_data(web::Data::new(datastore.clone()))
			.app_data(web::Data::new(sender.clone()))
			.configure(config)
	})
	.workers(2)
	.bind(("0.0.0.0", port))?
	.run()
	.await
	.map_err(|err| err.into())
}

#[tokio::main(worker_threads = 1)]
async fn main() {
	if cfg!(debug_assertions) {
		std::env::set_var("RUST_BACKTRACE", "1");
	}

	init().await;

	info!(
		version = PACKAGE_VERSION,
		"{}@{} by {}", PACKAGE_NAME, PACKAGE_VERSION, PACKAGE_AUTHORS
	);

	let (store_file, skip_clean, skip_validate, port) = {
		let config = CONFIG.read().await;
		(
			config.backend.store_file.clone(),
			config.backend.startup.skip_clean,
			config.backend.startup.skip_validate,
			config.backend.port,
		)
	};

	let store_path = PathBuf::from(
		&std::env::var("STORE_PATH").map(PathBuf::from).unwrap_or(
			get_file_path_and_create_directory(&store_file, &DATA_DIR)
				.expect("should retrieve data directory"),
		),
	);

	debug!(store_path = store_path.to_str(), "Loading from store");
	let store: Arc<Store> =
		Arc::new(Store::read_from_file(store_path.clone()).unwrap_or(Store::new(Some(store_path))));

	if !skip_clean {
		store.clean_up();
	}


	if !skip_validate && !store.validate() {
		error!("Validity of the data is not guaranteed. Cannot run backend...");
		exit(1);
	}

	info!("Database Started...");
	info!("Starting Program...");

	let (txtx, _) = broadcast::channel::<CardEvent>(1);

	let server_future = run_web_server(port, store.clone(), txtx.clone()).fuse();

	let watch_future = start_watch(store.clone(), txtx.clone()).fuse();

	pin_mut!(server_future, watch_future);

	select! {
		result = server_future => match result {
			Ok(_) => info!("Server ran to completion..."),
			Err(err) => error!(%err, "Server exited with error")
		},
		result = watch_future => match result {
			Ok(_) => info!("Watch ran to completion.."),
			Err(err) => error!(%err, "Watch exited with error"),
		},
	};

	info!("Saving Database");
	if let Err(err) = store.write_to_file() {
		error!(%err, "Failed to write datastore to file");
	}

	info!("Exiting...");
}
