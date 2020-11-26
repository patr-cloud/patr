#[path = "./api.vicara.co/mod.rs"]
mod api_vicara_co;
#[path = "./assets.vicara.co/mod.rs"]
mod assets_vicara_co;
#[path = "./auth.vicara.co/mod.rs"]
mod auth_vicara_co;

use crate::{
	app::{create_eve_app, App},
	utils::{EveContext, EveMiddleware},
};
use eve_rs::App as EveApp;

pub fn create_sub_app(app: App) -> EveApp<EveContext, EveMiddleware, App> {
	let mut sub_app = create_eve_app(app.clone());

	sub_app.use_middleware(
		"/",
		&[
			EveMiddleware::DomainRouter(
				String::from("api.vicara.co"),
				Box::new(api_vicara_co::create_sub_app(app.clone())),
			),
			EveMiddleware::DomainRouter(
				String::from("assets.vicara.co"),
				Box::new(assets_vicara_co::create_sub_app(app.clone())),
			),
			EveMiddleware::DomainRouter(
				String::from("auth.vicara.co"),
				Box::new(auth_vicara_co::create_sub_app(app)),
			),
		],
	);

	sub_app
}
