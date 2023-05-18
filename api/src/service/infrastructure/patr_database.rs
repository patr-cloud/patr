use api_models::{
	models::workspace::infrastructure::database::{
		PatrDatabaseEngine,
		PatrDatabasePlan,
		PatrDatabaseStatus,
	},
	utils::Uuid,
};
use chrono::Utc;
use eve_rs::AsError;
use rand::{distributions::Alphanumeric, thread_rng, Rng};

use crate::{
	db,
	error,
	models::rbac,
	service,
	utils::{constants::free_limits, validator, Error},
	Database,
};

pub async fn create_patr_database_in_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	name: &str,
	db_name: &str,
	engine: &PatrDatabaseEngine,
	database_plan: &PatrDatabasePlan,
	region_id: &Uuid,
	workspace_id: &Uuid,
	request_id: &Uuid,
	replica_numbers: i32,
) -> Result<Uuid, Error> {
	log::trace!("request_id: {} - Creating a patr database with name: {} and db_name: {} on DigitalOcean App platform with request_id: {}",
		request_id,
		name,
		db_name,
		request_id
	);

	log::trace!(
		"request_id: {} - Validating the patr database name",
		request_id
	);
	if !validator::is_database_name_valid(db_name) {
		log::trace!("request_id: {} - Database name is invalid. Rejecting create request", request_id);
		return Err(Error::empty()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string()));
	}

	log::trace!("request_id: {} - Generating new resource", request_id);
	let database_id = db::generate_new_resource_id(connection).await?;

	// validate whether the deployment region is ready
	let region_details = db::get_region_by_id(connection, region_id)
		.await?
		.status(400)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	if !region_details.is_ready() {
		return Err(Error::empty()
			.status(500)
			.body(error!(REGION_NOT_READY_YET).to_string()));
	}

	check_patr_database_creation_limit(
		connection,
		workspace_id,
		region_details.is_byoc_region(),
		database_plan,
		request_id,
	)
	.await?;

	let creation_time = Utc::now();

	db::create_resource(
		connection,
		&database_id,
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(rbac::resource_types::PATR_DATABASE)
			.unwrap(),
		workspace_id,
		&creation_time,
	)
	.await?;

	if !region_details.is_byoc_region() {
		db::start_patr_database_usage_history(
			connection,
			workspace_id,
			&database_id,
			database_plan,
			&creation_time,
		)
		.await?;
	}

	let password = thread_rng()
		.sample_iter(&Alphanumeric)
		.take(8)
		.map(char::from)
		.collect::<String>();

	let (version, port, username) = match engine {
		PatrDatabaseEngine::Postgres => ("12", 5432, "postgres"),
		PatrDatabaseEngine::Mysql => ("5", 3306, "root"),
		PatrDatabaseEngine::Mongo => ("4", 27017, "admin"),
		PatrDatabaseEngine::Redis => ("6", 6379, "default"),
	};

	log::trace!(
		"request_id: {} - Creating entry for newly created patr database",
		request_id
	);
	db::create_patr_database(
		connection,
		&database_id,
		name,
		workspace_id,
		region_id,
		db_name,
		engine,
		version,
		database_plan,
		&format!("db-{database_id}"),
		port,
		username,
		&password,
		replica_numbers,
	)
	.await?;
	log::trace!("request_id: {} - Resource generation complete", request_id);

	let kubeconfig =
		service::get_kubernetes_config_for_region(connection, region_id)
			.await?
			.0;

	match engine {
		PatrDatabaseEngine::Postgres => {
			service::patch_kubernetes_psql_database(
				workspace_id,
				&database_id,
				&password,
				database_plan,
				kubeconfig,
				request_id,
				replica_numbers,
			)
			.await?;
		}
		PatrDatabaseEngine::Mysql => {}
		PatrDatabaseEngine::Mongo => {}
		PatrDatabaseEngine::Redis => {}
	}

	Ok(database_id)
}

pub async fn modify_patr_database(
	connection: &mut <Database as sqlx::Database>::Connection,
	database_id: &Uuid,
	request_id: &Uuid,
	replica_numbers: i32,
) -> Result<(), Error> {
	log::trace!(
		"request_id: {} - Modifying patr database with id: {}",
		request_id,
		database_id
	);

	let database = db::get_patr_database_by_id(connection, database_id)
		.await?
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let kubeconfig =
		service::get_kubernetes_config_for_region(connection, &database.region)
			.await?
			.0;

	db::update_replica_number(connection, database_id, replica_numbers).await?;

	match database.engine {
		PatrDatabaseEngine::Postgres => {
			service::patch_kubernetes_psql_database(
				&database.workspace_id,
				&database.id,
				&database.password,
				&database.database_plan,
				kubeconfig,
				request_id,
				replica_numbers,
			)
			.await?;
		}
		PatrDatabaseEngine::Mysql => {}
		PatrDatabaseEngine::Mongo => {}
		PatrDatabaseEngine::Redis => {}
	}
	Ok(())
}

pub async fn delete_patr_database(
	connection: &mut <Database as sqlx::Database>::Connection,
	database_id: &Uuid,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!(
		"request_id: {} - Deleting patr database with id: {}",
		request_id,
		database_id
	);
	let database = db::get_patr_database_by_id(connection, database_id)
		.await?
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	db::delete_patr_database(connection, database_id, &Utc::now()).await?;

	if db::get_region_by_id(connection, &database.region)
		.await?
		.status(500)?
		.is_patr_region()
	{
		db::stop_patr_database_usage_history(
			connection,
			database_id,
			&Utc::now(),
		)
		.await?;
	}

	let kubeconfig =
		service::get_kubernetes_config_for_region(connection, &database.region)
			.await?
			.0;

	// now delete the database from k8s
	match database.engine {
		PatrDatabaseEngine::Postgres => {
			service::delete_kubernetes_psql_database(
				&database.workspace_id,
				&database.id,
				kubeconfig,
				request_id,
			)
			.await?;
		}
		PatrDatabaseEngine::Mysql => {}
		PatrDatabaseEngine::Mongo => {}
		PatrDatabaseEngine::Redis => {}
	}

	Ok(())
}

pub async fn get_patr_database_status(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	database_id: &Uuid,
	request_id: &Uuid,
) -> Result<PatrDatabaseStatus, Error> {
	log::trace!("Check patr database status: {database_id}");
	let database = db::get_patr_database_by_id(connection, database_id)
		.await?
		.status(500)?;

	let kubeconfig =
		service::get_kubernetes_config_for_region(connection, &database.region)
			.await?
			.0;

	let status = service::get_kubernetes_database_status(
		workspace_id,
		database_id,
		kubeconfig,
		request_id,
	)
	.await?;

	Ok(status)
}

async fn check_patr_database_creation_limit(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	is_byoc_region: bool,
	database_plan: &PatrDatabasePlan,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!("request_id: {request_id} - Checking whether new database creation is limited");

	if is_byoc_region {
		// if byoc, then don't need to check free/paid/total limits
		// as this database is going to be deployed on their cluster
		return Ok(());
	}

	let current_database_count =
		db::get_all_patr_database_for_workspace(connection, workspace_id)
			.await?
			.len();

	let card_added =
		db::get_default_payment_method_for_workspace(connection, workspace_id)
			.await?
			.is_some();
	if !card_added &&
		(current_database_count >= free_limits::PATR_DATABASE_COUNT ||
			database_plan != &PatrDatabasePlan::db_1r_1c_10v)
	{
		log::info!("request_id: {request_id} - Free database limit reached and card is not added");
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
