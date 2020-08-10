
#[path = "./api.bytesonus.com/mod.rs"]
mod api_bytesonus_com;
#[path = "./assets.bytesonus.com/mod.rs"]
mod assets_bytesonus_com;
#[path = "./auth.bytesonus.com/mod.rs"]
mod auth_bytesonus_com;

use crate::{
	app::{create_eve_app, App},
	utils::{EveContext, EveMiddleware},
};
use express_rs::App as EveApp;

pub fn create_sub_app(app: App) -> EveApp<EveContext, EveMiddleware, App> {
	let mut sub_app = create_eve_app(app.clone());

	sub_app.use_middleware(
		"/",
		&[
			EveMiddleware::DomainRouter(
				String::from("api.bytesonus.com"),
				Box::new(api_bytesonus_com::create_sub_app(app.clone())),
			),
			EveMiddleware::DomainRouter(
				String::from("assets.bytesonus.com"),
				Box::new(assets_bytesonus_com::create_sub_app(app.clone())),
			),
			EveMiddleware::DomainRouter(
				String::from("auth.bytesonus.com"),
				Box::new(auth_bytesonus_com::create_sub_app(app)),
			),
		],
	);

	sub_app
}
