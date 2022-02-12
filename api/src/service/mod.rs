//! # Overview
//! This module is for setting up the configuration of the API, when the API
//! starts running the data in form of [`App`] struct is transferred to the
//! APP variable which is then can be used in other parts of API
//!
//! [`App`]: App
mod auth;
mod docker_registry;
mod domain;
mod infrastructure;
mod metrics;
mod notifier;
mod user;
mod utils;
mod workspace;

use api_models::utils::Uuid;
use lapin::{
	options::QueueDeclareOptions,
	types::FieldTable,
	Channel,
	Connection,
	ConnectionProperties,
};

pub use self::{
	auth::*,
	docker_registry::*,
	domain::*,
	infrastructure::*,
	metrics::*,
	notifier::*,
	user::*,
	utils::*,
	workspace::*,
};
use crate::{
	app::App,
	utils::{settings::Settings, Error},
};

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

pub(super) async fn get_rabbitmq_connection_channel(
	config: &Settings,
	request_id: &Uuid,
) -> Result<(Channel, Connection), Error> {
	log::trace!("request_id: {} - Connecting to rabbitmq", request_id);
	let connection = Connection::connect(
		&format!(
			"amqp://{}:{}/%2f",
			config.rabbit_mq.host, config.rabbit_mq.port
		),
		ConnectionProperties::default(),
	)
	.await?;

	log::trace!("request_id: {} - Creating channel", request_id);
	let channel = connection.create_channel().await?;

	log::trace!("request_id: {} - Declaring queue", request_id);
	let _ = channel
		.queue_declare(
			"infrastructure",
			QueueDeclareOptions::default(),
			FieldTable::default(),
		)
		.await?;

	Ok((channel, connection))
}
