use api_models::{models::workspace::region::RegionStatus, utils::Uuid};
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
	build_machine_type_id: &Uuid,
	request_id: &Uuid,
) -> Result<Uuid, Error> {
	// validate inputs
	if !is_ci_runner_name_valid(name) {
		log::info!("request_id {} - Invalid runner name", request_id);
		return Err(Error::empty()
			.status(400)
			.body(error!(INVALID_RUNNER_NAME).to_string()));
	}

	// check whether region is allowed for workspace
	let region = db::get_all_regions_for_workspace(connection, workspace_id)
		.await?
		.into_iter()
		.find(|available_region| &available_region.id == region_id);
	if let Some(region_details) = region {
		log::info!("request_id {} - Region not ready yet", request_id);
		if !(region_details.status == RegionStatus::Active ||
			region_details.is_patr_region())
		{
			return Err(Error::empty()
				.status(500)
				.body(error!(REGION_NOT_READY_YET).to_string()));
		}
	} else {
		log::info!("request_id {} - Invalid region id", request_id);
		return Err(Error::empty()
			.status(400)
			.body(error!(RESOURCE_DOES_NOT_EXIST).to_string()));
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
		build_machine_type_id,
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

	// check whether runner is used by any repo
	let repos = db::list_active_repos_for_runner(connection, runner_id).await?;
	if !repos.is_empty() {
		log::info!("request_id {} - Runner is used by some repo", request_id);
		return Err(Error::empty()
			.status(400)
			.body(error!(RESOURCE_IN_USE).to_string()));
	}

	// check whether any build is currently-running / going-to-run in runner
	let queued_builds =
		db::list_queued_builds_for_runner(connection, runner_id).await?;
	if !queued_builds.is_empty() {
		log::info!("request_id {} - Runner is used by some builds", request_id);
		return Err(Error::empty()
			.status(400)
			.body(error!(RESOURCE_IN_USE).to_string()));
	}

	// validation success, now delete resource
	log::info!("request_id {} - Marking runner as deleted", request_id);
	db::mark_runner_as_deleted(connection, runner_id).await?;

	Ok(())
}
