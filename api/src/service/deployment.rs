use eve_rs::AsError;
use sqlx::{MySql, Transaction};

use crate::{
	db, error,
	models::db_mapping::DeploymentConfig,
	models::rbac,
	utils::{validator, Error},
};

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
			.body(error!(RESOURCE_DOES_NOT_EXISTS).to_string())?;
	}

	// get deployment details from deployment table

	Ok(())
}
