use std::ops::DerefMut;

use api_models::utils::{DateTime, Uuid};
use chrono::Utc;
use eve_rs::AsError;

use crate::{
	db::{
		self,
		ManagedDatabaseEngine,
		ManagedDatabasePlan,
		ManagedDatabaseStatus,
	},
	error,
	models::rbac,
	service::{
		self,
		infrastructure::{aws, digitalocean},
	},
	utils::{
		constants::free_limits,
		get_current_time_millis,
		settings::Settings,
		validator,
		Error,
	},
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

	log::trace!("request_id: {} - Checking resource limit", request_id);
	if super::resource_limit_crossed(connection, workspace_id, request_id)
		.await?
	{
		return Error::as_result()
			.status(400)
			.body(error!(RESOURCE_LIMIT_EXCEEDED).to_string())?;
	}

	log::trace!("request_id: {} - Checking database limit", request_id);
	if database_limit_crossed(connection, workspace_id, request_id).await? {
		return Error::as_result()
			.status(400)
			.body(error!(DATABASE_LIMIT_EXCEEDED).to_string())?;
	}

	db::create_resource(
		connection,
		&database_id,
		&format!("{}-database-{}", provider, database_id),
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(rbac::resource_types::MANAGED_DATABASE)
			.unwrap(),
		workspace_id,
		get_current_time_millis(),
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
		None,
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
		"aws" => {
			aws::create_managed_database_cluster(
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

	let (provider, region) = database
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
		"aws" => {
			log::trace!(
				"request_id: {} - deleting the deployment from aws",
				request_id
			);
			aws::delete_database(database_id, region, request_id).await?;
		}
		_ => {
			return Err(Error::empty()
				.status(500)
				.body(error!(SERVER_ERROR).to_string()));
		}
	}

	db::update_managed_database_name(
		connection,
		database_id,
		&format!("patr-deleted: {}-{}", database.name, database.id),
	)
	.await?;

	db::update_managed_database_status(
		connection,
		database_id,
		&ManagedDatabaseStatus::Deleted,
	)
	.await?;
	Ok(())
}

pub(super) async fn update_managed_database_status(
	database_id: &Uuid,
	status: &ManagedDatabaseStatus,
) -> Result<(), sqlx::Error> {
	let app = service::get_app();

	db::update_managed_database_status(
		app.database.acquire().await?.deref_mut(),
		database_id,
		status,
	)
	.await?;

	Ok(())
}

pub(super) async fn update_managed_database_credentials_for_database(
	database_id: &Uuid,
	host: &str,
	port: i32,
	username: &str,
	password: &str,
) -> Result<(), sqlx::Error> {
	let app = service::get_app();

	db::update_managed_database_credentials_for_database(
		app.database.acquire().await?.deref_mut(),
		database_id,
		host,
		port,
		username,
		password,
	)
	.await?;

	Ok(())
}

async fn database_limit_crossed(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	request_id: &Uuid,
) -> Result<bool, Error> {
	log::trace!(
		"request_id: {} - Checking if free limits are crossed",
		request_id
	);

	let workspace = db::get_workspace_info(connection, workspace_id)
		.await?
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	let workspace_info = db::get_workspace_info(connection, workspace_id)
		.await?
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	let current_databases =
		db::get_all_database_clusters_for_workspace(connection, workspace_id)
			.await?
			.len();

	if &(current_databases as i32) >= free_limits::FREE_MANAGED_DATABASE &&
		workspace_info.default_payment_method_id.is_none()
	{
		log::trace!("request_id: {} - Free limits are crossed", request_id);
		return Ok(true);
	}

	log::trace!(
		"request_id: {} - Checking if database limits are crossed",
		request_id
	);
	if current_databases + 1 > workspace.database_limit as usize {
		return Ok(true);
	}

	Ok(false)
}

pub async fn stop_database_subscription(
	connection: &mut <Database as sqlx::Database>::Connection,
	database_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!(
		"request_id: {} - Stopping subscription for database with id: {}",
		request_id,
		database_id
	);

	let database = db::get_managed_database_by_id(connection, database_id)
		.await?
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	let stop_time = &DateTime::from(Utc::now());

	let database_payment_history_id = if let Some(payment_history_id) =
		database.database_payment_history_id
	{
		payment_history_id
	} else {
		return Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string());
	};

	let database_payment_history = db::get_managed_database_payment_history_id(
		connection,
		&database_payment_history_id,
	)
	.await?
	.status(500)
	.body(error!(SERVER_ERROR).to_string())?;

	db::update_with_stop_database_payment_history(
		connection,
		&database_payment_history_id,
		Some(stop_time),
	)
	.await?;

	Ok(())
}

async fn start_database_subscription(
	connection: &mut <Database as sqlx::Database>::Connection,
	database_id: &Uuid,
	workspace_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!(
		"request_id: {} - Starting subscription for database with id: {}",
		request_id,
		database_id
	);

	let database = db::get_managed_database_by_id(connection, database_id)
		.await?
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	let db_payment_hist =
		db::generate_new_database_payment_history_id(connection).await?;

	db::add_database_payment_history(
		connection,
		&db_payment_hist,
		workspace_id,
		&database.id,
		&database.database_plan,
		&DateTime::from(Utc::now()),
		None,
	)
	.await?;

	db::update_database_with_payment_history_id(
		connection,
		&deployment.id,
		&db_payment_hist,
	)
	.await?;

	Ok(())
}

async fn update_database_subscription(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	deployment_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!(
		"request_id: {} - Updating subscription for deployment with id: {}",
		request_id,
		deployment_id
	);

	let database = db::get_managed_database_by_id(connection, database_id)
		.await?
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	let stop_time = &DateTime::from(Utc::now());

	let database_payment_history_id = if let Some(payment_history_id) =
		database.database_payment_history_id
	{
		payment_history_id
	} else {
		return Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string());
	};

	let database_payment_history = db::get_deployment_payment_history_id(
		connection,
		&database_payment_history_id,
	)
	.await?
	.status(500)
	.body(error!(SERVER_ERROR).to_string())?;

	db::update_with_stop_database_payment_history(
		connection,
		&database_payment_history_id,
		Some(stop_time),
	)
	.await?;

	let db_payment_hist =
		db::generate_new_deployment_payment_history_id(connection).await?;

	db::add_database_payment_history(
		connection,
		&db_payment_hist,
		workspace_id,
		&deployment.id,
		&deployment.machine_type,
		deployment.min_horizontal_scale as i32,
		&stop_time,
		None,
	)
	.await?;

	db::update_database_with_payment_history_id(
		connection,
		&database.id,
		&db_payment_hist,
	)
	.await?;

	Ok(())
}
