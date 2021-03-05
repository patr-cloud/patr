use crate::{
	app::{create_eve_app, App},
	utils::{EveContext, EveMiddleware},
};
use eve_rs::App as EveApp;

mod auth;
mod oauth2;
mod organisation;
mod user;

pub fn create_sub_app(app: &App) -> EveApp<EveContext, EveMiddleware, App> {
	let mut sub_app = create_eve_app(app);

	sub_app.use_sub_app("/auth", auth::create_sub_app(app));
	sub_app.use_sub_app("/user", user::create_sub_app(app));
	sub_app.use_sub_app("/organisation", organisation::create_sub_app(app));
	sub_app.use_sub_app("/oauth2", oauth2::create_sub_app(app));

	sub_app
}
