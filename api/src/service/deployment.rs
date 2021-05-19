use eve_rs::AsError;
use sqlx::{MySql, Transaction};

use crate::{
	db,
	error,
	models::{db_mapping::DeploymentConfig, rbac},
	utils::{validator, Error},
};

// function to return deployment config details
pub async fn get_deployment_config_by_id(
	connection: &mut Transaction<'_, MySql>,
	deployment_id: &[u8],
) -> Result<Option<DeploymentConfig>, Error> {
	// check if given deployment id is valid
	let deployment =
		db::get_deployment_by_id(connection, deployment_id).await?;
	if deployment.is_none() {
		Error::as_result()
			.status(400)
			.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;
	}
	let deployment = deployment.unwrap();
	let port_list =
		db::get_ports_for_deployment(connection, deployment_id).await?;
	let variable_list =
		db::get_variables_for_deployment(connection, deployment_id).await?;
	let volume_mount_list =
		db::get_volume_mounts_for_deployment(connection, deployment_id).await?;

	Ok(Some(DeploymentConfig {
		id: deployment.id,
		name: deployment.name,
		registry: deployment.registry,
		image_name: deployment.image_name,
		image_tag: deployment.image_tag,
		domain_id: deployment.domain_id,
		sub_domain: deployment.sub_domain,
		path: deployment.path,
		port_list,
		env_variable_list: variable_list,
		volume_mount_list,
	}))
}
