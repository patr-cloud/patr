mod auth;
mod deployment;
mod docker;
mod domain;
mod notifier;
mod organisation;
mod portus;
mod user;
mod utils;

pub use auth::*;
pub use deployment::*;
pub use docker::*;
pub use domain::*;
pub use notifier::*;
pub use organisation::*;
pub use portus::*;
pub use user::*;
pub use utils::*;

use crate::{app::App, utils::settings::Settings};

static APP_SETTINGS: once_cell::sync::OnceCell<App> =
	once_cell::sync::OnceCell::new();

pub fn initialize(app: &App) {
	let mut app = app.clone();
	app.config.password_pepper = base64::encode(&app.config.password_pepper);
	APP_SETTINGS.set(app).expect("unable to set app settings");
}

pub(super) fn get_settings() -> &'static Settings {
	&APP_SETTINGS
		.get()
		.expect("unable to get app settings")
		.config
}

pub(super) fn get_app() -> &'static App {
	APP_SETTINGS.get().expect("unable to get app settings")
}
