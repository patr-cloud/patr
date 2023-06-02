use eve_rs::App as EveApp;

use crate::{
	app::{create_eve_app, App},
	utils::{Error, EveContext, EveMiddleware},
};

pub fn create_sub_app(
	app: &App,
) -> EveApp<EveContext, EveMiddleware, App, Error> {
	// TODO Populate auth sub-apps here
	create_eve_app(app)
}
