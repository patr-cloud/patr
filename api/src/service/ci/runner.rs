use api_models::utils::Uuid;
use chrono::Utc;
use eve_rs::AsError;

use crate::{
	db,
	error,
	models::rbac,
	utils::{validator::is_ci_runner_name_valid, Error},
	Database,
};

pub async fn create_runner_for_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	name: &str,
	region_id: &Uuid,
	cpu: i32,
	ram: i32,
	volume: i32,
	request_id: &Uuid,
) -> Result<Uuid, Error> {
	// validate inputs
	if !is_ci_runner_name_valid(name) {
		log::info!("request_id {} - Invalid runner name", request_id);
		return Err(Error::empty()
			.status(400)
			.body(error!(INVALID_RUNNER_NAME).to_string()));
	}

	if cpu <= 0 || ram <= 0 || volume <= 0 {
		log::info!("request_id {} - Invalid resource values", request_id);
		return Err(Error::empty().status(400));
	}

	let is_region_valid =
		db::get_all_deployment_regions_for_workspace(connection, workspace_id)
			.await?
			.into_iter()
			.any(|available_region| &available_region.id == region_id);

	if !is_region_valid {
		log::info!("request_id {} - Invalid region id", request_id);
		return Err(Error::empty().status(400));
	}

	// validation success, now create resource
	log::info!("request_id {} - Creating resource for runner", request_id);
	let runner_id = db::generate_new_resource_id(connection).await?;
	db::create_resource(
		connection,
		&runner_id,
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(rbac::resource_types::CI_RUNNER)
			.unwrap(),
		workspace_id,
		&Utc::now(),
	)
	.await?;

	log::info!("request_id {} - Adding entry in runner table", request_id);
	db::create_runner_for_workspace(
		connection,
		&runner_id,
		name,
		workspace_id,
		region_id,
		cpu,
		ram,
		volume,
	)
	.await?;

	Ok(runner_id)
}

pub async fn update_runner(
	connection: &mut <Database as sqlx::Database>::Connection,
	runner_id: &Uuid,
	name: &str,
	request_id: &Uuid,
) -> Result<(), Error> {
	// validate inputs
	if !is_ci_runner_name_valid(name) {
		log::info!("request_id {} - Invalid runner name", request_id);
		return Err(Error::empty()
			.status(400)
			.body(error!(INVALID_RUNNER_NAME).to_string()));
	}

	let _runner_exists = db::get_runner_by_id(connection, runner_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	// todo: allow updating resource values too

	// validation success, now create resource
	db::update_runner(connection, runner_id, name).await?;

	Ok(())
}

pub async fn delete_runner(
	connection: &mut <Database as sqlx::Database>::Connection,
	runner_id: &Uuid,
	request_id: &Uuid,
) -> Result<(), Error> {
	// validate inputs
	let _runner_exists = db::get_runner_by_id(connection, runner_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	// todo: check whether there is no repo using this as a runner
	// todo: how to handle if something is already queued

	// validation success, now create resource
	log::info!("request_id {} - Marking runner as deleted", request_id);
	db::mark_runner_as_deleted(connection, runner_id).await?;

	Ok(())
}
