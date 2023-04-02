use api_models::models::GetVersionResponse;
use axum::{routing::get, Error, Json, Router};

use crate::{app::App, utils::constants};

mod auth;
mod user;
mod webhook;
mod workspace;

pub fn create_sub_route(app: &App) -> Router<App> {
	let router = Router::new()
		.nest("/auth", auth::create_sub_route(app))
		.nest("/user", user::create_sub_route(app))
		.nest("/workspace", workspace::create_sub_route(app))
		.nest("/webhook", webhook::create_sub_route(app))
		.route("/version", get(get_version_number));

	router
}

async fn get_version_number() -> Result<Json<GetVersionResponse>, Error> {
	let version = GetVersionResponse {
		version: constants::DATABASE_VERSION.to_string(),
	};
	Ok(Json(version))
}
