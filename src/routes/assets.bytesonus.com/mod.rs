use crate::{
	app::{create_eve_app, App},
	utils::{EveContext, EveMiddleware},
};
use express_rs::App as EveApp;

pub fn create_sub_app(app: App) -> EveApp<EveContext, EveMiddleware, App> {
	let sub_app = create_eve_app(app.clone());

	// TODO Populate assets sub-apps here

	sub_app
}
