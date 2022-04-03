use api_models::{models::workspace::infrastructure::secret::*, utils::Uuid};
use vaultrs::{
	client::{VaultClient, VaultClientSettingsBuilder},
	kv2,
};

use crate::{
	db,
	models::rbac,
	utils::{get_current_time_millis, settings::Settings, Error},
	Database,
};

pub async fn create_new_secret_in_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	name: &str,
	secret: &str,
	_config: &Settings,
	request_id: &Uuid,
) -> Result<Uuid, Error> {
	let secret_id = db::generate_new_resource_id(connection).await?;

	log::trace!("request_id: {} - Creating resource.", request_id);
	// db::create_resource(
	// 	connection,
	// 	&secret_id,
	// 	&format!("Secret: {}", secret_id),
	// 	rbac::RESOURCE_TYPES
	// 		.get()
	// 		.unwrap()
	// 		.get(rbac::resource_types::SECRET_ID)
	// 		.unwrap(),
	// 	workspace_id,
	// 	get_current_time_millis(),
	// )
	// .await?;

	log::trace!("request_id: {} - Creating database entry", request_id);

	db::create_new_secret_in_workspace(
		connection,
		&secret_id,
		secret,
		workspace_id,
	)
	.await?;

	log::trace!("request_id: {} - Creating secret in vault", request_id);

	let client = VaultClient::new(
		VaultClientSettingsBuilder::default()
			.address("https://127.0.0.1:8200")
			.token("s.w0ZJ8DQAQtvO0r9xsfB6Mgbv") // move this hard coded token to config file
			.build()
			.unwrap(),
	)
	.unwrap();

	let secret = SecretBody {
		name: name.to_string(),
		secret: secret.to_string(),
	};

	kv2::set(
		&client,
		"secret",
		format!("{}/{}", workspace_id.as_str(), secret_id.as_str()).as_str(),
		&secret,
	)
	.await?;

	let secret: SecretBody =
		kv2::read(&client, workspace_id.as_str(), secret_id.as_str()).await?;
	println!("{}", secret.secret);
	log::trace!("request_id: {} - Created secret.", request_id);

	Ok(secret_id)
}

pub async fn delete_secret_in_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	_workspace_id: &Uuid,
	secret_id: &Uuid,
	_config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!("request_id: {} - Deleting secret with id: {} on database with request_id: {}",
		request_id,
		secret_id,
		request_id
	);
	db::update_secret_name(
		connection,
		secret_id,
		&format!("patr-deleted-{}", secret_id),
	)
	.await?;

	Ok(())
}
