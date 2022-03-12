use api_models::utils::Uuid;

use crate::{
	db,
	models::rbac,
	service::infrastructure::kubernetes,
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
	let secret_id = db::generate_new_resource_id(connection).await?;

	log::trace!("request_id: {} - Creating resource.", request_id);
	db::create_resource(
		connection,
		&secret_id,
		&format!("Secret: {}", secret_id),
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(rbac::resource_types::SECRET_ID)
			.unwrap(),
		workspace_id,
		get_current_time_millis(),
	)
	.await?;

	log::trace!("request_id: {} - Creating database entry", request_id);
	if let Some(deployment_id) = deployment_id {
		db::create_new_secret_for_deployment(
			connection,
			&secret_id,
			name,
			workspace_id,
			deployment_id,
		)
		.await?;
	} else {
		db::create_new_secret_in_workspace(
			connection,
			&secret_id,
			secret,
			workspace_id,
		)
		.await?;
	}

	log::trace!("request_id: {} - Creating secret.", request_id);
	kubernetes::update_kuberenetes_secrets(
		workspace_id,
		&secret_id,
		secret,
		config,
		request_id,
	)
	.await?;
	log::trace!("request_id: {} - Created secret.", request_id);

	Ok(secret_id)
}

pub async fn delete_secret_in_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	secret_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!("request_id: {} - Deleting secret with id: {} on Kubernetes with request_id: {}",
		request_id,
		secret_id,
		request_id
	);

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

	log::trace!("request_id: {} - Deleting secret with id: {} on Kubernetes with request_id: {}",
		request_id,
		secret_id,
		request_id
	);
	kubernetes::delete_kubernetes_secret(
		workspace_id,
		secret_id,
		config,
		request_id,
	)
	.await?;

	Ok(())
}
