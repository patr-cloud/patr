use api_models::utils::Uuid;
use eve_rs::AsError;
use vaultrs::{
	client::{VaultClient, VaultClientSettingsBuilder},
	kv2,
};

use crate::{
	db,
	error,
	models::rbac,
	utils::{get_current_time_millis, settings::Settings, Error},
	Database,
};

pub async fn create_new_secret_in_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	name: &str,
	secret_value: &str,
	config: &Settings,
	request_id: &Uuid,
) -> Result<Uuid, Error> {
	let resource_id = db::generate_new_resource_id(connection).await?;

	log::trace!("request_id: {} - Creating resource.", request_id);
	db::create_resource(
		connection,
		&resource_id,
		&format!("Secret: {}", name),
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(rbac::resource_types::SECRET)
			.unwrap(),
		workspace_id,
		get_current_time_millis(),
	)
	.await?;

	log::trace!("request_id: {} - Creating database entry", request_id);

	db::create_new_secret_in_workspace(
		connection,
		&resource_id,
		name,
		workspace_id,
	)
	.await?;

	log::trace!("request_id: {} - Creating secret in vault", request_id);

	let client = VaultClient::new(
		VaultClientSettingsBuilder::default()
			.address(&config.vault.address)
			.token(&config.vault.token)
			.build()?,
	)?;

	kv2::set(
		&client,
		"secret",
		&format!("{}/{}", workspace_id.as_str(), resource_id.as_str()),
		&[("data", secret_value)],
	)
	.await?;

	log::trace!("request_id: {} - Created secret.", request_id);

	Ok(resource_id)
}

#[allow(dead_code)]
pub async fn create_new_secret_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	deployment_id: &Uuid,
	name: &str,
	secret_value: &str,
	config: &Settings,
	request_id: &Uuid,
) -> Result<Uuid, Error> {
	let resource_id = db::generate_new_resource_id(connection).await?;

	log::trace!("request_id: {} - Creating resource.", request_id);
	db::create_resource(
		connection,
		&resource_id,
		&format!("Secret: {}", name),
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(rbac::resource_types::SECRET)
			.unwrap(),
		workspace_id,
		get_current_time_millis(),
	)
	.await?;

	log::trace!("request_id: {} - Creating database entry", request_id);

	db::create_new_secret_for_deployment(
		connection,
		&resource_id,
		name,
		workspace_id,
		deployment_id,
	)
	.await?;

	log::trace!("request_id: {} - Creating secret in vault", request_id);

	let client = VaultClient::new(
		VaultClientSettingsBuilder::default()
			.address(&config.vault.address)
			.token(&config.vault.token)
			.build()?,
	)?;

	kv2::set(
		&client,
		"secret",
		&format!("{}/{}", workspace_id, resource_id),
		&[("data", secret_value)],
	)
	.await?;

	log::trace!("request_id: {} - Created secret.", request_id);

	Ok(resource_id)
}

pub async fn update_workspace_secret(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	secret_id: &Uuid,
	name: Option<&str>,
	value: Option<&str>,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!(
		"request_id: {} - Updating secret with id: {}",
		request_id,
		secret_id,
	);
	db::get_secret_by_id(connection, secret_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	if let Some(name) = name {
		db::update_secret_name(connection, secret_id, name).await?;
	}

	if let Some(value) = value {
		log::trace!(
			"request_id: {} - Updating secret value in vault",
			request_id
		);

		let client = VaultClient::new(
			VaultClientSettingsBuilder::default()
				.address(&config.vault.address)
				.token(&config.vault.token)
				.build()?,
		)?;

		kv2::set(
			&client,
			"secret",
			&format!("{}/{}", workspace_id, secret_id),
			&[("data", value)],
		)
		.await?;
	}

	Ok(())
}

pub async fn delete_secret_in_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	secret_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!(
		"request_id: {} - Deleting secret with id: {}",
		request_id,
		secret_id,
	);

	db::get_secret_by_id(connection, secret_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let client = VaultClient::new(
		VaultClientSettingsBuilder::default()
			.address(&config.vault.address)
			.token(&config.vault.token)
			.build()?,
	)?;

	log::trace!(
		"request_id: {} - Deleting secret with id: {} from vault",
		request_id,
		secret_id,
	);

	let metadata = kv2::read_metadata(
		&client,
		"secret",
		&format!("{}/{}", workspace_id, secret_id),
	)
	.await?;

	kv2::destroy_versions(
		&client,
		"secret",
		&format!("{}/{}", workspace_id.as_str(), secret_id.as_str()),
		metadata
			.versions
			.keys()
			.into_iter()
			.filter_map(|version| version.parse::<u64>().ok())
			.collect(),
	)
	.await?;

	log::trace!(
		"request_id: {} - Deleted secret with id: {} from vault",
		request_id,
		secret_id,
	);

	log::trace!(
		"request_id: {} - Deleting secret with id: {} from database",
		request_id,
		secret_id,
	);

	db::update_secret_name(
		connection,
		secret_id,
		&format!("patr-deleted-{}", secret_id),
	)
	.await?;

	log::trace!(
		"request_id: {} - Deleted secret with id: {} from databae",
		request_id,
		secret_id,
	);

	Ok(())
}
