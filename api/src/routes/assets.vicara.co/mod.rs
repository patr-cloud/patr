use crate::{
	app::{create_eve_app, App},
	utils::{ErrorData, EveContext, EveMiddleware},
};
use eve_rs::App as EveApp;

pub fn create_sub_app(
	app: &App,
) -> EveApp<EveContext, EveMiddleware, App, ErrorData> {
	// TODO Populate assets sub-apps here
	create_eve_app(app)
}
