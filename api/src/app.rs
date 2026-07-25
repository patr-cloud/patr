use std::{
	fmt::{self, Debug, Formatter},
	net::{IpAddr, SocketAddr},
};

use apalis_postgres::PostgresStorage;
use axum::extract::FromRef;
use models::{RequestUserData, prelude::*};
use preprocess::Preprocessable;
use rustis::client::Client as RedisClient;
use tokio::net::TcpListener;

use crate::{prelude::*, utils::config::AppConfig, worker::WorkerTaskType};

/// Sets up the router and starts the server.
#[instrument(skip(state))]
pub async fn serve(state: &AppState) {
	cfg_if! {
		if #[cfg(all(feature = "cloud", debug_assertions))] {
			use futures::FutureExt;
			let api_listener = TcpListener::bind(state.config.server.bind_address)
				.await
				.unwrap();

			info!(
				"API server running on http://{}",
				api_listener.local_addr().unwrap()
			);

			let app_listener = TcpListener::bind(SocketAddr::from((
				state.config.server.bind_address.ip(),
				state.config.server.bind_address.port() + 1,
			)))
			.await
			.unwrap();

			info!(
				"Frontend server running on http://{}",
				app_listener.local_addr().unwrap()
			);

			let registry_listener = TcpListener::bind(SocketAddr::from((
				state.config.server.bind_address.ip(),
				state.config.server.bind_address.port() + 2,
			)))
			.await
			.unwrap();

			info!(
				"Registry server running on http://{}",
				registry_listener.local_addr().unwrap()
			);

			let loki_listener = TcpListener::bind(SocketAddr::from((
				state.config.server.bind_address.ip(),
				state.config.server.bind_address.port() + 3,
			)))
			.await
			.unwrap();

			info!(
				"Loki server running on http://{}",
				loki_listener.local_addr().unwrap()
			);

			let assets_listener = TcpListener::bind(SocketAddr::from((
				state.config.server.bind_address.ip(),
				state.config.server.bind_address.port() + 4,
			)))
			.await
			.unwrap();

			info!(
				"Assets server running on http://{}",
				assets_listener.local_addr().unwrap()
			);

			let mimir_listener = TcpListener::bind(SocketAddr::from((
				state.config.server.bind_address.ip(),
				state.config.server.bind_address.port() + 5,
			)))
			.await
			.unwrap();

			info!(
				"Mimir server running on http://{}",
				mimir_listener.local_addr().unwrap()
			);

			futures::future::join_all([
				async {
					axum::serve(
						api_listener,
						crate::routes::api_patr_cloud::setup_routes(state, ClientType::ApiToken)
							.await
							.into_make_service_with_connect_info::<SocketAddr>(),
					)
					.with_graceful_shutdown(crate::exit_signal())
					.await
					.unwrap();
				}
				.boxed(),
				async {
					axum::serve(
						app_listener,
						crate::routes::app_patr_cloud::setup_routes(state)
							.await
							.into_make_service_with_connect_info::<SocketAddr>(),
					)
					.with_graceful_shutdown(crate::exit_signal())
					.await
					.unwrap();
				}
				.boxed(),
				async {
					axum::serve(
						registry_listener,
						crate::routes::registry_patr_cloud::setup_routes(state)
							.await
							.into_make_service_with_connect_info::<SocketAddr>(),
					)
					.with_graceful_shutdown(crate::exit_signal())
					.await
					.unwrap();
				}
				.boxed(),
				async {
					axum::serve(
						loki_listener,
						crate::routes::loki_patr_cloud::setup_routes(state)
							.await
							.into_make_service_with_connect_info::<SocketAddr>(),
					)
					.with_graceful_shutdown(crate::exit_signal())
					.await
					.unwrap();
				}
				.boxed(),
				async {
					axum::serve(
						assets_listener,
						crate::routes::assets_patr_cloud::setup_routes(state)
							.await
							.into_make_service_with_connect_info::<SocketAddr>(),
					)
					.with_graceful_shutdown(crate::exit_signal())
					.await
					.unwrap();
				}
				.boxed(),
				async {
					axum::serve(
						mimir_listener,
						crate::routes::mimir_patr_cloud::setup_routes(state)
							.await
							.into_make_service_with_connect_info::<SocketAddr>(),
					)
					.with_graceful_shutdown(crate::exit_signal())
					.await
					.unwrap();
				}
				.boxed(),
			])
			.await;
		} else {
			let tcp_listener = TcpListener::bind(state.config.server.bind_address)
				.await
				.unwrap();

			info!(
				"Listening for connections on http://{}",
				tcp_listener.local_addr().unwrap()
			);

			axum::serve(
				tcp_listener,
				crate::routes::setup_routes(state)
					.await
					.into_make_service_with_connect_info::<SocketAddr>(),
			)
			.with_graceful_shutdown(crate::exit_signal())
			.await
			.unwrap();
		}
	}
}

#[derive(Clone, FromRef)]
/// The global state of the application.
/// This will contain the database connection and other global state.
pub struct AppState {
	/// The database connection.
	/// **Note:** This is NOT a transaction. The request object will contain a
	/// transaction.
	pub database: sqlx::Pool<DatabaseType>,
	/// The redis connection.
	/// **Note:** This is NOT a transaction. The request object will contain a
	/// transaction.
	pub redis: RedisClient,
	/// The application configuration.
	pub config: AppConfig,
	/// The background worker storage.
	pub worker: PostgresStorage<WorkerTaskType>,
}

impl Debug for AppState {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		f.debug_struct("AppState")
			.field("database", &self.database)
			.field("redis", &"[RedisClient]")
			.finish()
	}
}

/// A request object that is passed through the tower layers and services for
/// endpoints that do not require authentication
pub struct UnprocessedAppRequest<'a, E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// The Endpoint that the request is being made for. This would ideally be
	/// parsed to have all the data needed to process a request
	pub request: ApiRequest<E>,
	/// The database transaction for the request. In case the request returns
	/// an Error, this transaction will be automatically rolled back.
	pub database: &'a mut DatabaseTransaction,
	/// The redis transaction for the request. In case the request returns
	/// an Error, this transaction will be automatically rolled back.
	pub redis: &'a mut RedisClient,
	/// The IP address of the client that made the request.
	pub client_ip: IpAddr,
	/// The application state
	pub state: AppState,
}

/// A request object that is passed through the tower layers and services for
/// endpoints that do not require authentication
pub struct AppRequest<'a, E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// The Endpoint that the request is being made for. This would ideally be
	/// parsed and preprocessed to have all the data needed to process a request
	pub request: ProcessedApiRequest<E>,
	/// The database transaction for the request. In case the request returns
	/// an Error, this transaction will be automatically rolled back.
	pub database: &'a mut DatabaseTransaction,
	/// The redis transaction for the request. In case the request returns
	/// an Error, this transaction will be automatically rolled back.
	pub redis: &'a mut RedisClient,
	/// The IP address of the client that made the request.
	pub client_ip: IpAddr,
	/// The application state
	pub state: AppState,
}

/// A request object that is passed through the tower layers and services for
/// endpoints that require authentication. This will contain the user data of
/// the current authenticated user.
pub struct AuthenticatedAppRequest<'a, E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// The Endpoint that the request is being made for. This would ideally be
	/// parsed to have all the data needed to process a request
	pub request: ProcessedApiRequest<E>,
	/// The database transaction for the request. In case the request returns
	/// an Error, this transaction will be automatically rolled back.
	pub database: &'a mut DatabaseTransaction,
	/// The redis transaction for the request. In case the request returns
	/// an Error, this transaction will be automatically rolled back.
	pub redis: &'a mut RedisClient,
	/// The IP address of the client that made the request.
	pub client_ip: IpAddr,
	/// The user data of the current authenticated user.
	pub user_data: RequestUserData,
	/// The application state
	pub state: AppState,
}
