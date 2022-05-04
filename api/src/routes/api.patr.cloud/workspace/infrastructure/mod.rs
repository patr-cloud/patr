use eve_rs::App as EveApp;

use crate::{
	app::{create_eve_app, App},
	utils::{ErrorData, EveContext, EveMiddleware},
};

mod deployment;
mod deployment_machine_type;
mod deployment_region;
mod managed_database;
mod managed_url;
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
	sub_app.use_sub_app("/managed-url", managed_url::create_sub_app(app));
	sub_app.use_sub_app("/static-site", static_site::create_sub_app(app));
	sub_app.use_sub_app("/region", deployment_region::create_sub_app(app));
	sub_app.use_sub_app(
		"/machine-type",
		deployment_machine_type::create_sub_app(app),
	);

	sub_app
}
