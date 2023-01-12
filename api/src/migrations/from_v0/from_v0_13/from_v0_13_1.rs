use api_models::{
	models::workspace::infrastructure::deployment::DeploymentStatus,
	utils::Uuid,
};
use chrono::Utc;
use sqlx::Row;

use crate::{
	db,
	migrate_query as query,
	service,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	delete_deployment_with_invalid_image_name(connection, config).await?;
	validate_image_name_for_deployment(connection, config).await?;
	Ok(())
}

pub(super) async fn delete_deployment_with_invalid_image_name(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	let deployments = query!(
		r#"
		SELECT
			*
		FROM
			deployment
		WHERE
			image_name IS NOT NULL AND
			image_name !~ '^(([a-z0-9]+)(((?:[._]|__|[-]*)([a-z0-9]+))*)?)(((\/)(([a-z0-9]+)(((?:[._]|__|[-]*)([a-z0-9]+))*)?))*)?$';
		"#
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| (
		row.get::<Uuid, _>("id"),
		row.get::<Uuid, _>("workspace_id"),
		row.get::<Uuid, _>("region"),
	))
	.collect::<Vec<_>>();

	let request_id = Uuid::new_v4();
	for (deployment_id, workspace_id, region_id) in deployments {
		if service::is_deployed_on_patr_cluster(connection, &region_id).await? {
			db::stop_deployment_usage_history(
				connection,
				&deployment_id,
				&Utc::now(),
			)
			.await?;
		}

		db::update_deployment_status(
			connection,
			&deployment_id,
			&DeploymentStatus::Stopped,
		)
		.await?;

		db::update_deployment_image_name(
			connection,
			&deployment_id,
			"undefined",
		)
		.await?;

		let kube_config = service::get_kubernetes_config_for_region(
			connection, &region_id, config,
		)
		.await?;

		service::delete_kubernetes_deployment(
			&workspace_id,
			&deployment_id,
			kube_config,
			&request_id,
		)
		.await?
	}

	Ok(())
}

pub(super) async fn validate_image_name_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE deployment
		ADD CONSTRAINT deployment_chk_image_name_is_valid
		CHECK (
			image_name::TEXT ~ '^(([a-z0-9]+)(((?:[._]|__|[-]*)([a-z0-9]+))*)?)(((\/)(([a-z0-9]+)(((?:[._]|__|[-]*)([a-z0-9]+))*)?))*)?$'
		);
	"#
	)
	.execute(&mut *connection)
	.await?;
	Ok(())
}
