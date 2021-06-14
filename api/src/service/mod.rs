//! # Overview
//! This module is for setting up the configuration of the API, when the API
//! starts running the data in form of [`Settings`] struct is transferred to the
//! APP_SETTINGS variable which is then can be used in other parts of API
//!
//! [`Settings`]: Settings
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

use crate::utils::settings::Settings;

/// stores the configuration of the whole API
static APP_SETTINGS: once_cell::sync::OnceCell<Settings> =
	once_cell::sync::OnceCell::new();

/// # Description
/// This function is used to insert into [`APP_SETTINGS`] after the API starts
///
/// # Arguments
/// * `config` - An instance of struct [`Settings`]
///
/// [`Settings`]: Settings
/// [`API_SETTINGS`]: API_SETTINGS
pub fn initialize(config: &Settings) {
	let mut config = config.clone();
	config.password_pepper = base64::encode(&config.password_pepper);
	APP_SETTINGS
		.set(config)
		.expect("unable to set app settings");
}
/// # Description
/// This function is used to retrieve the configuration of API
///
/// # Returns
/// It returns a ```&'static``` variable [`APP_SETTINGS`]
/// [`APP_SETTINGS`]: APP_SETTINGS
pub(super) fn get_config() -> &'static Settings {
	APP_SETTINGS.get().expect("unable to get app settings")
}
