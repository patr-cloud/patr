mod auth;
mod docker;
mod domain;
mod notifier;
mod organisation;
mod portus;
mod user;
mod utils;

pub use auth::*;
pub use docker::*;
pub use domain::*;
pub use notifier::*;
pub use organisation::*;
pub use portus::*;
pub use user::*;
pub use utils::*;

use crate::utils::settings::Settings;

static APP_SETTINGS: once_cell::sync::OnceCell<Settings> =
	once_cell::sync::OnceCell::new();

pub fn initialize(config: &Settings) {
	let mut config = config.clone();
	config.password_pepper = base64::encode(&config.password_pepper);
	APP_SETTINGS
		.set(config)
		.expect("unable to set app settings");
}

pub(super) fn get_config() -> &'static Settings {
	APP_SETTINGS.get().expect("unable to get app settings")
}
