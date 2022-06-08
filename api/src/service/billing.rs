use api_models::utils::Uuid;
use chrono::Utc;
use eve_rs::AsError;

use crate::{db, utils::Error, Database};

pub async fn create_billable_service_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	deployment_id: &Uuid,
	active: bool,
) -> Result<Uuid, Error> {
	let deployment = db::get_deployment_by_id(connection, &deployment_id)
		.await?
		.status(500)?;

	let plan_info = db::get_plan_by_deployment_machine_type(
		connection,
		&deployment.machine_type,
	)
	.await?
	.status(500)?;

	let service_id = db::create_billable_service(
		connection,
		&plan_info.id,
		&workspace_id,
		plan_info.price,
		Some(deployment.min_horizontal_scale as i32),
		&plan_info.product_info_id,
		deployment_id,
		Utc::now().into(),
		active,
	)
	.await?;

	Ok(service_id)
}
