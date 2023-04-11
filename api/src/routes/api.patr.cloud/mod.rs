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

/// # Description
/// This function is used to create a sub app for every endpoint listed. It
/// creates an eve app which binds the endpoint with functions. This file
/// contains major enpoints of the API, and all other endpoints will come under
/// this
///
/// # Arguments
/// * `app` - an object of type [`App`] which contains all the configuration of
///   api including the
/// database connections.
///
/// # Returns
/// this function returns `EveApp<EveContext, EveMiddleware, App, ErrorData>`
/// containing context, middleware, object of [`App`] and Error
///
/// [`App`]: App
pub fn create_sub_app() -> Router<App> {
	Router::new()
		.merge(auth::create_sub_app())
		.merge(user::create_sub_app())
		.merge(workspace::create_sub_app())
		.merge(webhook::create_sub_app())
		.mount_dto(get_version_number)
}

async fn get_version_number(
	_: DecodedRequest<GetVersionRequest>,
) -> Result<GetVersionResponse, Error> {
	Ok(GetVersionResponse {
		version: constants::DATABASE_VERSION.to_string(),
	})
}
