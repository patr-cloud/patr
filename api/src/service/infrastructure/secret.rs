use api_models::utils::Uuid;
use vaultrs::{
	client::{VaultClient, VaultClientSettingsBuilder},
	kv2,
};

use crate::{
	db,
	models::{db_mapping::SecretBody, rbac},
	utils::{get_current_time_millis, settings::Settings, Error},
	Database,
};

pub async fn create_new_secret_in_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	deployment_id: Option<&Uuid>,
	name: &str,
	secret: &str,
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

	let secret = SecretBody {
		name: name.to_string(),
		secret: secret.to_string(),
	};

	kv2::set(
		&client,
		"secret",
		&format!("{}/{}", workspace_id.as_str(), resource_id.as_str()),
		&secret,
	)
	.await?;

	log::trace!("request_id: {} - Created secret.", request_id);

	Ok(resource_id)
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

	kv2::delete_latest(
		&client,
		"secret",
		&format!("{}/{}", workspace_id.as_str(), secret_id.as_str()),
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
