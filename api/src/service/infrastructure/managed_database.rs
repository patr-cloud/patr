use api_models::{
	models::workspace::infrastructure::database::{
		ManagedDatabaseEngine,
		ManagedDatabaseStatus,
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
	utils::{constants::free_limits, settings::Settings, validator, Error},
	Database,
};

pub async fn create_managed_database_in_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	name: &str,
	engine: &ManagedDatabaseEngine,
	database_plan_id: &Uuid,
	region_id: &Uuid,
	workspace_id: &Uuid,
	request_id: &Uuid,
) -> Result<(Uuid, String), Error> {
	log::trace!(
		"request_id: {} - Creating a patr database with name: {}",
		request_id,
		name,
	);

	log::trace!(
		"request_id: {} - Validating the patr database name",
		request_id
	);
	if !validator::is_database_name_valid(name) {
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

	check_managed_database_creation_limit(
		connection,
		workspace_id,
		region_details.is_byoc_region(),
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
			.get(rbac::resource_types::MANAGED_DATABASE)
			.unwrap(),
		workspace_id,
		&creation_time,
	)
	.await?;

	if !region_details.is_byoc_region() {
		db::start_managed_database_usage_history(
			connection,
			workspace_id,
			&database_id,
			database_plan_id,
			&creation_time,
		)
		.await?;
	}

	let password = thread_rng()
		.sample_iter(&Alphanumeric)
		.take(8)
		.map(char::from)
		.collect::<String>();

	let username = match engine {
		ManagedDatabaseEngine::Postgres => "postgres",
		ManagedDatabaseEngine::Mysql => "root",
		ManagedDatabaseEngine::Mongo => "root",
		ManagedDatabaseEngine::Redis => "root",
	};

	log::trace!(
		"request_id: {} - Creating entry for newly created patr database",
		request_id
	);
	db::create_managed_database(
		connection,
		&database_id,
		name,
		workspace_id,
		region_id,
		engine,
		database_plan_id,
		username,
	)
	.await?;
	log::trace!("request_id: {} - Resource generation complete", request_id);

	let kubeconfig =
		service::get_kubernetes_config_for_region(connection, region_id)
			.await?
			.0;

	let database_plan =
		db::get_database_plan_by_id(connection, database_plan_id).await?;

	match engine {
		ManagedDatabaseEngine::Postgres => {
			service::patch_kubernetes_psql_database(
				workspace_id,
				&database_id,
				&database_plan,
				kubeconfig,
				request_id,
			)
			.await?;
		}
		ManagedDatabaseEngine::Mongo => {
			service::patch_kubernetes_mongo_database(
				workspace_id,
				&database_id,
				&database_plan,
				kubeconfig,
				request_id,
				true,
				true,
			)
			.await?;
		}
		ManagedDatabaseEngine::Redis => {
			service::patch_kubernetes_redis_database(
				workspace_id,
				&database_id,
				&password,
				&database_plan,
				kubeconfig,
				request_id,
			)
			.await?;
		}
		ManagedDatabaseEngine::Mysql => {
			service::patch_kubernetes_mysql_database(
				workspace_id,
				&database_id,
				&database_plan,
				kubeconfig,
				request_id,
			)
			.await?;
		}
	}

	Ok((database_id, password))
}

pub async fn delete_managed_database(
	connection: &mut <Database as sqlx::Database>::Connection,
	database_id: &Uuid,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!(
		"request_id: {} - Deleting patr database with id: {}",
		request_id,
		database_id
	);
	let database = db::get_managed_database_by_id(connection, database_id)
		.await?
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	db::delete_managed_database(connection, database_id, &Utc::now()).await?;

	if db::get_region_by_id(connection, &database.region)
		.await?
		.status(500)?
		.is_patr_region()
	{
		db::stop_managed_database_usage_history(
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
	service::delete_kubernetes_database(
		&database.workspace_id,
		&database.id,
		kubeconfig,
		request_id,
	)
	.await?;

	Ok(())
}

async fn check_managed_database_creation_limit(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	is_byoc_region: bool,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!("request_id: {request_id} - Checking whether new database creation is limited");

	if is_byoc_region {
		// if byoc, then don't need to check free/paid/total limits
		// as this database is going to be deployed on their cluster
		return Ok(());
	}

	let current_database_count =
		db::get_all_managed_database_for_workspace(connection, workspace_id)
			.await?
			.len();

	let card_added =
		db::get_default_payment_method_for_workspace(connection, workspace_id)
			.await?
			.is_some();

	if !card_added &&
		(current_database_count > free_limits::MANAGED_DATABASE_COUNT)
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

pub async fn change_database_password(
	connection: &mut <Database as sqlx::Database>::Connection,
	database_id: &Uuid,
	request_id: &Uuid,
	new_password: &String,
	config: &Settings,
) -> Result<(), Error> {
	let database = db::get_managed_database_by_id(connection, database_id)
		.await?
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let database_plan =
		db::get_database_plan_by_id(connection, &database.database_plan_id)
			.await?;

	let kubeconfig =
		service::get_kubernetes_config_for_region(connection, &database.region)
			.await?
			.0;

	match database.engine {
		ManagedDatabaseEngine::Postgres => {
			service::change_psql_database_password(
				&database.workspace_id,
				&database.id,
				kubeconfig,
				request_id,
				new_password,
			)
			.await
		}
		ManagedDatabaseEngine::Mysql => {
			service::change_mysql_database_password(
				&database.workspace_id,
				&database.id,
				kubeconfig,
				request_id,
				new_password,
			)
			.await
		}
		ManagedDatabaseEngine::Mongo => {
			log::trace!("request_id: {request_id} - Changing Mongo statefulset config to disable auth");

			service::change_mongo_database_auth(
				&database.workspace_id,
				&database.id,
				kubeconfig,
				request_id,
				&database_plan,
				false,
				false,
			)
			.await?;

			db::update_managed_database_status(
				connection,
				database_id,
				&ManagedDatabaseStatus::Creating,
			)
			.await?;

			log::trace!(
				"request_id: {request_id} - Queuing for mongo password change"
			);
			service::queue_status_check_update_and_change_mongo_database_password(
				&database.workspace_id,
				database_id,
				config,
				request_id,
				new_password,
			)
			.await?;
		}
		ManagedDatabaseEngine::Redis => {
			service::change_redis_database_password(
				&database.workspace_id,
				&database.id,
				kubeconfig,
				request_id,
				new_password,
			)
			.await
		}
	}
}
