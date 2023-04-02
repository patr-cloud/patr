mod api_token;
mod login;
mod user;

use std::net::SocketAddr;

use axum::{
	extract::{ConnectInfo, State},
	http::Request,
	middleware::Next,
	response::Response,
	Router,
};

pub use self::{api_token::*, login::*, user::*};
use crate::app::App;

pub fn create_sub_route(app: &App) -> Router<App> {
	let router = Router::new()
		.nest("/logins", login::create_sub_route(app))
		.nest("/api-tokens", api_token::create_sub_route(app))
		.nest("/", user::create_sub_route(app));

	router
}
