use eve_rs::App as EveApp;

use crate::{
	app::{create_eve_app, App},
	utils::{ErrorData, EveContext, EveMiddleware},
};

#[allow(clippy::module_inception)]
mod deployment;
mod managed_database;
mod static_site;

pub fn create_sub_app(
	app: &App,
) -> EveApp<EveContext, EveMiddleware, App, ErrorData> {
	let mut sub_app = create_eve_app(app);

	sub_app.use_sub_app("/deployment", deployment::create_sub_app(app));
	sub_app.use_sub_app(
		"/managed-database",
		managed_database::create_sub_app(app),
	);
	sub_app.use_sub_app("/static-site", static_site::create_sub_app(app));

	sub_app
}
