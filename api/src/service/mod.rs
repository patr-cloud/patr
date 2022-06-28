//! # Overview
//! This module is for setting up the configuration of the API, when the API
//! starts running the data in form of [`App`] struct is transferred to the
//! APP variable which is then can be used in other parts of API
//!
//! [`App`]: App
mod auth;
mod billing;
mod ci;
mod docker_registry;
mod domain;
mod infrastructure;
mod metrics;
mod notifier;
mod rabbitmq;
mod user;
mod utils;
mod workspace;

pub use self::{
	auth::*,
	billing::*,
	ci::*,
	docker_registry::*,
	domain::*,
	infrastructure::*,
	metrics::*,
	notifier::*,
	rabbitmq::*,
	user::*,
	utils::*,
	workspace::*,
};
use crate::{app::App, utils::settings::Settings};

/// stores the configuration and database of the whole API
static APP: once_cell::sync::OnceCell<App> = once_cell::sync::OnceCell::new();

/// # Description
/// This function is used to insert into [`APP`] after the API starts
///
/// # Arguments
/// * `app` - An instance of struct [`App`]
///
/// [`App`]: App
/// [`APP`]: APP
pub fn initialize(app: &App) {
	let mut app = app.clone();
	app.config.password_pepper = base64::encode(&app.config.password_pepper);
	APP.set(app).expect("unable to set app settings");
}

/// # Description
/// This function is used to retrieve the configuration of API
///
/// # Returns
/// It returns the `Settings` of the ```&'static``` variable [`APP`]
/// [`APP`]: APP
pub(super) fn get_settings() -> &'static Settings {
	&APP.get().expect("unable to get app settings").config
}

/// # Description
/// This function is used to retrieve the app data of API
///
/// # Returns
/// It returns the ```&'static``` variable [`APP`]
/// [`APP`]: APP
pub(super) fn get_app() -> &'static App {
	APP.get().expect("unable to get app")
}
