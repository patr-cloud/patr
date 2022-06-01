mod aws;
mod deployment;
mod digitalocean;
mod kubernetes;
mod managed_database;
mod managed_url;
mod secret;
mod static_site;

use std::ops::DerefMut;

use api_models::{
	models::workspace::infrastructure::database::ManagedDatabaseStatus,
	utils::Uuid,
};

pub use self::{
	deployment::*,
	kubernetes::*,
	managed_database::*,
	managed_url::*,
	secret::*,
	static_site::*,
};
use crate::{
	db::{self},
	service,
};

async fn update_managed_database_status(
	database_id: &Uuid,
	status: &ManagedDatabaseStatus,
) -> Result<(), sqlx::Error> {
	let app = service::get_app();

	db::update_managed_database_status(
		app.database.acquire().await?.deref_mut(),
		database_id,
		status,
	)
	.await?;

	Ok(())
}

async fn update_managed_database_credentials_for_database(
	database_id: &Uuid,
	host: &str,
	port: i32,
	username: &str,
	password: &str,
) -> Result<(), sqlx::Error> {
	let app = service::get_app();

	db::update_managed_database_credentials_for_database(
		app.database.acquire().await?.deref_mut(),
		database_id,
		host,
		port,
		username,
		password,
	)
	.await?;

	Ok(())
}
