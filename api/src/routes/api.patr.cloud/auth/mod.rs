mod auth;
mod docker_registry;

use std::net::SocketAddr;

use axum::{
	extract::{ConnectInfo, State},
	http::Request,
	middleware::Next,
	response::Response,
	Router,
};

pub use self::{auth::*, docker_registry::*};
use crate::app::App;

pub fn create_sub_route(app: &App) -> Router<App> {
	Router::new().nest("/", auth::create_sub_route(app)).nest(
		"/docker-registry-token",
		docker_registry::create_sub_route(app),
	)
}
