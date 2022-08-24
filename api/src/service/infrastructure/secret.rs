use std::collections::BTreeMap;

use api_models::utils::{DateTime, Uuid};
use chrono::Utc;
use eve_rs::AsError;
use vaultrs::{
	client::{VaultClient, VaultClientSettingsBuilder},
	error::ClientError,
	kv2,
};

use crate::{
	db,
	error,
	models::rbac,
	utils::{settings::Settings, Error},
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
	log::trace!("request_id: {} - Checking resource limit", request_id);
	if super::resource_limit_crossed(connection, workspace_id, request_id)
		.await?
	{
		return Error::as_result()
			.status(400)
			.body(error!(RESOURCE_LIMIT_EXCEEDED).to_string())?;
	}

	log::trace!("request_id: {} - Checking secret limit", request_id);
	if secret_limit_crossed(connection, workspace_id, request_id).await? {
		return Error::as_result()
			.status(400)
			.body(error!(SECRET_LIMIT_EXCEEDED).to_string())?;
	}

	let resource_id = db::generate_new_resource_id(connection).await?;

	let creation_time = Utc::now().into();
	log::trace!("request_id: {} - Creating resource", request_id);
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
		&creation_time,
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

	let secret_count =
		db::get_all_secrets_in_workspace(connection, workspace_id)
			.await?
			.len();
	db::update_secret_usage_history(
		connection,
		workspace_id,
		&(secret_count as i32),
		&DateTime::from(creation_time),
	)
	.await?;

	log::trace!("request_id: {} - Getting vault client", request_id);
	let client = VaultClient::new(
		VaultClientSettingsBuilder::default()
			.address(&config.vault.address)
			.token(&config.vault.token)
			.build()?,
	)?;

	log::trace!("request_id: {} - Creating secret in vault", request_id);
	kv2::set(
		&client,
		"secret",
		&format!("{}/{}", workspace_id.as_str(), resource_id.as_str()),
		&[("data", secret_value)]
			.into_iter()
			.collect::<BTreeMap<_, _>>(),
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

	let creation_time = Utc::now().into();
	log::trace!("request_id: {} - Creating resource", request_id);
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
		&creation_time,
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

	let secret_count =
		db::get_all_secrets_in_workspace(connection, workspace_id)
			.await?
			.len();
	db::update_secret_usage_history(
		connection,
		workspace_id,
		&(secret_count as i32),
		&DateTime::from(creation_time),
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
		&[("data", secret_value)]
			.into_iter()
			.collect::<BTreeMap<_, _>>(),
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
			&[("data", value)].into_iter().collect::<BTreeMap<_, _>>(),
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

	let secret = db::get_secret_by_id(connection, secret_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	//TODO: check if the secret is connected to a deployment or not

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
	.await;

	if let Err(ClientError::APIError { code: 404, .. }) = metadata {
		// Secret does not exist in vault
	} else {
		kv2::destroy_versions(
			&client,
			"secret",
			&format!("{}/{}", workspace_id.as_str(), secret_id.as_str()),
			metadata?
				.versions
				.keys()
				.into_iter()
				.filter_map(|version| version.parse::<u64>().ok())
				.collect(),
		)
		.await?;
	}

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
		&format!("patr-deleted: {}@{}", secret_id, secret.name),
	)
	.await?;

	let secret_count =
		db::get_all_secrets_in_workspace(connection, workspace_id)
			.await?
			.len();
	db::update_secret_usage_history(
		connection,
		workspace_id,
		&(secret_count as i32),
		&DateTime::from(Utc::now()),
	)
	.await?;

	log::trace!(
		"request_id: {} - Deleted secret with id: {} from databae",
		request_id,
		secret_id,
	);

	Ok(())
}

async fn secret_limit_crossed(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	request_id: &Uuid,
) -> Result<bool, Error> {
	log::trace!(
		"request_id: {} - Checking if free limits are crossed",
		request_id
	);

	let workspace = db::get_workspace_info(connection, workspace_id)
		.await?
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	let current_secrets =
		db::get_all_secrets_in_workspace(connection, workspace_id)
			.await?
			.len();

	log::trace!(
		"request_id: {} - Checking if secret limits are crossed",
		request_id
	);
	if current_secrets + 1 > workspace.secret_limit as usize {
		return Ok(true);
	}

	Ok(false)
}
