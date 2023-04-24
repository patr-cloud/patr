use api_models::{
	models::{GetVersionRequest, GetVersionResponse},
	utils::{DecodedRequest, DtoRequestExt},
	Error,
};
use axum::Router;

use crate::{app::App, utils::constants};

mod auth;
mod user;
mod webhook;
mod workspace;

/// This function is used to create a router for every endpoint in this file
pub fn create_sub_app(app: &App) -> Router<App> {
	Router::new()
		.merge(auth::create_sub_app(app))
		.merge(user::create_sub_app(app))
		.merge(workspace::create_sub_app(app))
		.merge(webhook::create_sub_app(app))
		.mount_dto(get_version_number)
}

async fn get_version_number(
	_: DecodedRequest<GetVersionRequest>,
) -> Result<GetVersionResponse, Error> {
	Ok(GetVersionResponse {
		version: constants::DATABASE_VERSION.to_string(),
	})
}
