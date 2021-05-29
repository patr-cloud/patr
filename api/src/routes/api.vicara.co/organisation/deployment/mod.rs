use eve_rs::App as EveApp;

use crate::{
	app::{create_eve_app, App},
	utils::{ErrorData, EveContext, EveMiddleware},
};

mod deployment;
mod entry_point;
mod upgrade_path;

pub fn create_sub_app(
	app: &App,
) -> EveApp<EveContext, EveMiddleware, App, ErrorData> {
	let mut sub_app = create_eve_app(app);

	sub_app.use_sub_app("/deployment", deployment::create_sub_app(app));
	sub_app.use_sub_app("/entry-point", entry_point::create_sub_app(app));
	sub_app.use_sub_app("/upgrade-path", upgrade_path::create_sub_app(app));

	sub_app
}
