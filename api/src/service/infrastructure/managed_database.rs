use api_models::utils::{DateTime, Uuid};
use chrono::Utc;
use eve_rs::AsError;

use crate::{
	db::{self, ManagedDatabaseEngine, ManagedDatabasePlan},
	error,
	models::rbac,
	service::infrastructure::digitalocean,
	utils::{constants::free_limits, settings::Settings, validator, Error},
	Database,
};

pub async fn create_managed_database_in_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	name: &str,
	db_name: &str,
	engine: &ManagedDatabaseEngine,
	version: Option<&str>,
	num_nodes: Option<u64>,
	database_plan: &ManagedDatabasePlan,
	region: &str,
	workspace_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<Uuid, Error> {
	let databases =
		db::get_all_database_clusters_for_workspace(connection, workspace_id)
			.await?;

	if databases.len() > 3 {
		return Error::as_result()
			.status(400)
			.body(error!(MAX_LIMIT_REACHED).to_string())?;
	}

	log::trace!("request_id: {} - Creating a managed database on digitalocean with name: {} and db_name: {} on DigitalOcean App platform with request_id: {}",
		request_id,
		name,
		db_name,
		request_id
	);

	log::trace!(
		"request_id: {} - Validating the managed database name",
		request_id
	);
	if !validator::is_database_name_valid(db_name) {
		log::trace!("request_id: {} - Database name is invalid. Rejecting create request", request_id);
		return Err(Error::empty()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string()));
	}

	let (provider, region) = region
		.split_once('-')
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	log::trace!("request_id: {} - Generating new resource", request_id);
	let database_id = db::generate_new_resource_id(connection).await?;

	let version = match engine {
		ManagedDatabaseEngine::Postgres => version.unwrap_or("12"),
		ManagedDatabaseEngine::Mysql => version.unwrap_or("8"),
	};
	let num_nodes = num_nodes.unwrap_or(1);

	check_database_creation_limit(connection, workspace_id, request_id).await?;

	let creation_time = Utc::now();

	db::create_resource(
		connection,
		&database_id,
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(rbac::resource_types::MANAGED_DATABASE)
			.unwrap(),
		workspace_id,
		&creation_time,
	)
	.await?;

	db::start_database_usage_history(
		connection,
		workspace_id,
		&database_id,
		database_plan,
		&DateTime::from(creation_time),
	)
	.await?;

	log::trace!(
		"request_id: {} - Creating entry for newly created managed database",
		request_id
	);
	db::create_managed_database(
		connection,
		&database_id,
		name,
		db_name,
		engine,
		version,
		num_nodes,
		database_plan,
		&format!("{}-{}", provider, region),
		"",
		0,
		"",
		"",
		workspace_id,
	)
	.await?;
	log::trace!("request_id: {} - Resource generation complete", request_id);

	match provider {
		"do" => {
			digitalocean::create_managed_database_cluster(
				connection,
				&database_id,
				db_name,
				engine,
				version,
				num_nodes,
				database_plan,
				region,
				config,
				request_id,
			)
			.await?;
		}
		_ => {
			return Err(Error::empty()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string()));
		}
	}

	Ok(database_id)
}

pub async fn delete_managed_database(
	connection: &mut <Database as sqlx::Database>::Connection,
	database_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!(
		"request_id: {} - Deleting managed database with id: {}",
		request_id,
		database_id
	);
	let database = db::get_managed_database_by_id(connection, database_id)
		.await?
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let (provider, _) = database
		.region
		.split_once('-')
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	match provider {
		"do" => {
			log::trace!(
				"request_id: {} - Deleting the database from digitalocean",
				request_id
			);
			if let Some(digitalocean_db_id) = database.digitalocean_db_id {
				digitalocean::delete_database(
					&digitalocean_db_id,
					config,
					request_id,
				)
				.await?;
			}
		}
		_ => {
			return Err(Error::empty()
				.status(500)
				.body(error!(SERVER_ERROR).to_string()));
		}
	}

	db::delete_managed_database(connection, database_id, &Utc::now()).await?;

	db::stop_database_usage_history(connection, database_id, &Utc::now())
		.await?;
	Ok(())
}

async fn check_database_creation_limit(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!("request_id: {request_id} - Checking whether new database creation is limited");

	let current_database_count =
		db::get_all_database_clusters_for_workspace(connection, workspace_id)
			.await?
			.len();

	// check whether free limit is exceeded
	if current_database_count >= free_limits::DATABASE_COUNT as usize &&
		db::get_default_payment_method_for_workspace(
			connection,
			workspace_id,
		)
		.await?
		.is_none()
	{
		log::info!(
			"request_id: {request_id} - Free database limit reached and card is not added"
		);
		return Error::as_result()
			.status(400)
			.body(error!(CARDLESS_FREE_LIMIT_EXCEEDED).to_string())?;
	}

	// check whether max database limit is exceeded
	let max_database_limit = db::get_workspace_info(connection, workspace_id)
		.await?
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?
		.database_limit;
	if current_database_count >= max_database_limit as usize {
		log::info!(
			"request_id: {request_id} - Max database limit for workspace reached"
		);
		return Error::as_result()
			.status(400)
			.body(error!(DATABASE_LIMIT_EXCEEDED).to_string())?;
	}

	// check whether total resource limit is exceeded
	if super::resource_limit_crossed(connection, workspace_id, request_id)
		.await?
	{
		log::info!("request_id: {request_id} - Total resource limit exceeded");
		return Error::as_result()
			.status(400)
			.body(error!(RESOURCE_LIMIT_EXCEEDED).to_string())?;
	}

	Ok(())
}
