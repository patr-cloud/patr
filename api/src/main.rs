//! The main API server for Patr.

use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() {
	use api::{
		app,
		db,
		redis_publisher,
		utils::{self, config},
	};

	let config = config::parse_config();

	let (logger_provider, tracer_provider) = utils::setup_tracing(&config);

	tracing::info!("Config parsed. Running in {} mode", config.environment);

	tokio::spawn(async move {
		let ctrl_c = async {
			tokio::signal::ctrl_c()
				.await
				.expect("Failed to listen for SIGINT")
		};

		#[cfg(unix)]
		let terminate = async {
			tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
				.expect("failed to install signal handler")
				.recv()
				.await;
		};

		#[cfg(not(unix))]
		let terminate = std::future::pending::<()>();

		tokio::select! {
			_ = ctrl_c => (),
			_ = terminate => (),
		}

		tracing::info!("Received shutdown signal, cancelling all connections");
		api::GLOBAL_CANCEL_TOKEN
			.get_or_init(CancellationToken::new)
			.cancel();
	});

	let state = api::build_state(config).await;

	db::initialize(&state)
		.await
		.expect("error initializing database");

	futures::future::join(app::serve(&state), redis_publisher::run(&state)).await;

	utils::flush_tracing(logger_provider, tracer_provider);
}
