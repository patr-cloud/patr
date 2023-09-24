use std::{
	fmt::{self, Debug, Formatter},
	net::IpAddr,
};

use axum::{extract::FromRef, http::StatusCode, Router};
use models::{prelude::*, RequestUserData};
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

/// A request object that is passed through the tower layers and services for
/// endpoints that do not require authentication
pub struct AppRequest<'a, E>
where
	E: ApiEndpoint,
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
/// endpoints that require authentication. This will contain the user data of
/// the current authenticated user.
pub struct AuthenticatedAppRequest<'a, E>
where
	E: ApiEndpoint,
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
	/// The user data of the current authenticated user.
	pub user_data: RequestUserData,
	/// The application configuration.
	pub config: AppConfig,
}

/// A response object that is passed through the tower layers and services
#[derive(TypedBuilder)]
#[builder(field_defaults(setter(into)))]
pub struct AppResponse<E>
where
	E: ApiEndpoint,
{
	/// The status code of the response
	pub status_code: StatusCode,
	/// The headers of the response
	pub headers: E::ResponseHeaders,
	/// The body of the response
	pub body: E::ResponseBody,
}

impl<E> AppResponse<E>
where
	E: ApiEndpoint,
{
	/// Convert the response into a Result
	pub fn into_result(self) -> Result<Self, ErrorType> {
		Ok(self)
	}
}
