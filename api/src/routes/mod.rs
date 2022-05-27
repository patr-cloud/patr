#[path = "api.patr.cloud/mod.rs"]
mod api_patr_cloud;
#[path = "assets.patr.cloud/mod.rs"]
mod assets_patr_cloud;
#[path = "auth.patr.cloud/mod.rs"]
mod auth_patr_cloud;

use eve_rs::App as EveApp;

use crate::{
	app::{create_eve_app, App},
	utils::{ErrorData, EveContext, EveMiddleware},
};

pub fn create_sub_app(
	app: &App,
) -> EveApp<EveContext, EveMiddleware, App, ErrorData> {
	let mut sub_app = create_eve_app(app);

	if cfg!(debug_assertions) {
		sub_app.use_sub_app("/", api_patr_cloud::create_sub_app(app));
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
			],
		);
	}

	sub_app
}
