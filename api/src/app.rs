use std::{
	fmt::{self, Debug, Formatter},
	net::IpAddr,
};

use axum::{extract::FromRef, Router};
use models::{prelude::*, RequestUserData};
use preprocess::Preprocessable;
use rustis::client::Client as RedisClient;
use typed_builder::TypedBuilder;

use crate::{prelude::*, utils::config::AppConfig};

/// Sets up all the routes for the API
#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	crate::routes::setup_routes(state).await
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
}

impl Debug for AppState {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		f.debug_struct("AppState")
			.field("database", &self.database)
			.field("redis", &"[RedisClient]")
			.finish()
	}
}

/// This struct represents a preprocessed request to the API. It contains the
/// path, query, headers and preprocessed body of the request. This struct
/// provides a builder API to make it easier to construct requests.
#[derive(TypedBuilder)]
pub struct ProcessedApiRequest<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// The path of the request. This is the part of the URL after the domain
	/// and port.
	pub path: E::RequestPath,
	/// The query of the request. This is the part of the URL after the `?`.
	pub query: E::RequestQuery,
	/// The headers of the request.
	pub headers: E::RequestHeaders,
	/// The body of the request. This is the actual data that was sent by the
	/// client. Can be either JSON or Websockets.
	pub body: <E::RequestBody as Preprocessable>::Processed,
}

impl<E> TryFrom<ApiRequest<E>> for ProcessedApiRequest<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	type Error = preprocess::Error;

	fn try_from(value: ApiRequest<E>) -> Result<Self, Self::Error> {
		let ApiRequest {
			path,
			query,
			headers,
			body,
		} = value;
		Ok(ProcessedApiRequest {
			path,
			query,
			headers,
			body: body.preprocess()?,
		})
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
	/// The application configuration.
	pub config: AppConfig,
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
	/// The application configuration.
	pub config: AppConfig,
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
	/// The application configuration.
	pub config: AppConfig,
}
