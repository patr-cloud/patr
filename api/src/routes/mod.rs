#[path = "api.patr.cloud/mod.rs"]
mod api_patr_cloud;
#[path = "assets.patr.cloud/mod.rs"]
mod assets_patr_cloud;
#[path = "auth.patr.cloud/mod.rs"]
mod auth_patr_cloud;

use axum::Router;

use crate::app::App;

pub async fn create_sub_route(app: &App) -> Router {
	Router::new().nest("/", api_patr_cloud::create_sub_route(app))
}

pub fn get_request_ip_address(context: &EveContext) -> String {
	let cf_connecting_ip = context.get_header("CF-Connecting-IP");
	let x_real_ip = context.get_header("X-Real-IP");
	let x_forwarded_for =
		context.get_header("X-Forwarded-For").and_then(|value| {
			value.split(',').next().map(|ip| ip.trim().to_string())
		});
	let ip = context.get_ip().to_string();

	cf_connecting_ip
		.or(x_real_ip)
		.or(x_forwarded_for)
		.unwrap_or(ip)
}
