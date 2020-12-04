use crate::{
	app::{create_eve_app, App},
	utils::{EveContext, EveMiddleware},
};
use eve_rs::App as EveApp;

pub fn create_sub_app(app: &App) -> EveApp<EveContext, EveMiddleware, App> {
	// TODO Populate assets sub-apps here
	create_eve_app(app)
}
