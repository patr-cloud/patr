use std::fmt::{self, Debug, Formatter};

use axum::{extract::FromRef, http::StatusCode, Router};
use models::{utils::ApiRequest, ApiEndpoint, ErrorType};
use rustis::client::{Client as RedisClient, Transaction as RedisTransaction};
use sea_orm::{DatabaseConnection, DatabaseTransaction};
use typed_builder::TypedBuilder;

use crate::{prelude::*, utils::config::AppConfig};

#[instrument(skip(state))]
pub fn setup_routes(state: &AppState) -> Router {
	crate::routes::setup_routes(state)
}

#[derive(Clone, FromRef)]
/// The global state of the application.
/// This will contain the database connection and other global state.
pub struct AppState {
	/// The database connection.
	/// **Note:** This is NOT a transaction. The request object will contain a
	/// transaction.
	pub database: DatabaseConnection,
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
			.field("redis", &"[RedisConnection]")
			.finish()
	}
}

/// A request object that is passed through the tower layers and services
pub struct AppRequest<E>
where
	E: ApiEndpoint,
{
	/// The Endpoint that the request is being made for. This would ideally be
	/// parsed to have all the data needed to process a request
	pub request: ApiRequest<E>,
	/// The database transaction for the request. In case the request returns
	/// an Error, this transaction will be automatically rolled back.
	pub database: DatabaseTransaction,
	/// The redis transaction for the request. In case the request returns
	/// an Error, this transaction will be automatically rolled back.
	pub redis: RedisTransaction,
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
