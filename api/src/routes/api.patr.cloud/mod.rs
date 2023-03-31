use axum::Router;

use crate::app::App;

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
pub fn create_sub_route(app: &App) -> Router {
	let router = Router::new()
		.nest("/auth", auth::create_sub_route(app))
		.nest("/user", user::create_sub_route(app))
		.nest("/workspace", workspace::create_sub_route(app))
		.nest("/webhook", webhook::create_sub_route(app));
	// .route("/version", get(get_version_number));

	router
}

// async fn get_version_number(
// State(state): State<App>,
// ) -> Result<EveContext, Error> {
// context.success(GetVersionResponse {
// version: constants::DATABASE_VERSION.to_string(),
// });
// Ok(context)
// }
