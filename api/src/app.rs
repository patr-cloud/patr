use std::fmt::Debug;

use axum::{
	extract::{FromRef, FromRequestParts, State},
	http::{Request, request::Parts},
	response::Response,
	middleware::Next,
	Router,
};
use models::{utils::{ApiResponse, ApiErrorResponse, IntoAxumResponse}, ErrorType};
use rustis::client::{Client as RedisClient, Transaction as RedisTransaction};
use sea_orm::{DatabaseConnection, DatabaseTransaction, TransactionTrait};

use crate::prelude::*;

#[derive(Clone, FromRef)]
pub struct AppState {
	pub database: DatabaseConnection,
	pub redis: RedisClient,
}

pub struct AppConnection<'a> {
	pub database: DatabaseTransaction,
	pub redis: RedisTransaction<'a>,
}

#[axum::async_trait]
impl<'a, S> FromRequestParts<S> for AppConnection<'a> {
	type Rejection = ErrorType;

	async fn from_request_parts(
		parts: &mut Parts,
		_: &S,
	) -> Result<Self, Self::Rejection> {
		parts.extensions.get::<AppConnection>().ok_or_else(|| {
			error!("Failed to get AppConnection from request extensions");
			ApiResponse::internal_error("Failed to get AppConnection")
		})
	}
}

impl Debug for AppState {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("AppState")
			.field("database", &self.database)
			.field("redis", &"[RedisConnection]")
			.finish()
	}
}

#[instrument(skip(state))]
pub fn setup_routes(state: &AppState) -> Router {
	crate::routes::setup_routes(state)
}
