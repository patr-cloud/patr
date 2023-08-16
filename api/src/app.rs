use std::fmt::Debug;

use axum::{extract::FromRef, Router};
use rustis::client::Client as RedisClient;
use sea_orm::DatabaseConnection;

#[derive(Clone, FromRef)]
pub struct AppState {
	pub database: DatabaseConnection,
	pub redis: RedisClient,
}

impl Debug for AppState {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("AppState")
			.field("database", &self.database)
			.field("redis", &"[RedisConnection]")
			.finish()
	}
}

pub fn setup_routes(state: &AppState) -> Router {
	crate::routes::setup_routes(state)
}
