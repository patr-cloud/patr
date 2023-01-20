mod git_provider;
mod runner;

use eve_rs::App as EveApp;

use crate::{
	app::{create_eve_app, App},
	utils::{ErrorData, EveContext, EveMiddleware},
};

pub fn create_sub_app(
	app: &App,
) -> EveApp<EveContext, EveMiddleware, App, ErrorData> {
	let mut sub_app = create_eve_app(app);

	sub_app.use_sub_app("/git-provider", git_provider::create_sub_app(app));
	sub_app.use_sub_app("/runner", runner::create_sub_app(app));

	sub_app
}
