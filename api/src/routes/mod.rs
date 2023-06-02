#[path = "api.patr.cloud/mod.rs"]
mod api_patr_cloud;
#[path = "assets.patr.cloud/mod.rs"]
mod assets_patr_cloud;
#[path = "auth.patr.cloud/mod.rs"]
mod auth_patr_cloud;
#[path = "loki.patr.cloud/mod.rs"]
mod loki_patr_cloud;
#[path = "mimir.patr.cloud/mod.rs"]
mod mimir_patr_cloud;
#[path = "vault.patr.cloud/mod.rs"]
mod vault_patr_cloud;

use eve_rs::{App as EveApp, Context};
use reqwest::header::HeaderName;

use crate::{
	app::{create_eve_app, App},
	utils::{Error, EveContext, EveMiddleware},
};

pub fn create_sub_app(
	app: &App,
) -> EveApp<EveContext, EveMiddleware, App, Error> {
	let mut sub_app = create_eve_app(app);

	if cfg!(debug_assertions) {
		sub_app.use_sub_app("/", api_patr_cloud::create_sub_app(app));
		sub_app.use_sub_app("/v1", vault_patr_cloud::create_sub_app(app));
		sub_app.use_sub_app("/loki-host", loki_patr_cloud::create_sub_app(app));
		sub_app
			.use_sub_app("/mimir-host", mimir_patr_cloud::create_sub_app(app));
	} else {
		sub_app.use_middleware(
			"/",
			[
				EveMiddleware::DomainRouter(
					String::from("api.patr.cloud"),
					Box::new(api_patr_cloud::create_sub_app(app)),
				),
				EveMiddleware::DomainRouter(
					String::from("assets.patr.cloud"),
					Box::new(assets_patr_cloud::create_sub_app(app)),
				),
				EveMiddleware::DomainRouter(
					String::from("auth.patr.cloud"),
					Box::new(auth_patr_cloud::create_sub_app(app)),
				),
				EveMiddleware::DomainRouter(
					String::from("vault.patr.cloud"),
					Box::new(vault_patr_cloud::create_sub_app(app)),
				),
				EveMiddleware::DomainRouter(
					app.config.loki.host.clone(),
					Box::new(loki_patr_cloud::create_sub_app(app)),
				),
				EveMiddleware::DomainRouter(
					app.config.mimir.host.clone(),
					Box::new(mimir_patr_cloud::create_sub_app(app)),
				),
			],
		);
	}

	sub_app
}

pub fn get_request_ip_address(context: &EveContext) -> String {
	let cf_connecting_ip =
		context.get_header(HeaderName::from_static("CF-Connecting-IP"));
	let x_real_ip = context.get_header(HeaderName::from_static("X-Real-IP"));
	let x_forwarded_for = context
		.get_header(HeaderName::from_static("X-Forwarded-For"))
		.and_then(|value| {
			value.split(',').next().map(|ip| ip.trim().to_string())
		});
	let ip = context.get_ip().to_string();

	cf_connecting_ip
		.or(x_real_ip)
		.or(x_forwarded_for)
		.unwrap_or(ip)
}
