use eve_rs::App as EveApp;

use crate::{
	app::{create_eve_app, App},
	utils::{ErrorData, EveContext, EveMiddleware},
};

pub fn create_sub_route(
	app: &App,
) -> EveApp<EveContext, EveMiddleware, App, ErrorData> {
	// TODO Populate assets sub-apps here
	create_eve_app(app)
}
