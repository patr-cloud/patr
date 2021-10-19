use std::ops::DerefMut;

use eve_rs::AsError;
use uuid::Uuid;

use crate::{
	db,
	error,
	models::{
		db_mapping::{
			CloudPlatform,
			ManagedDatabaseEngine,
			ManagedDatabasePlan,
			ManagedDatabaseStatus,
		},
		rbac,
	},
	service::{
		self,
		deployment::{aws, digitalocean},
	},
	utils::{get_current_time_millis, settings::Settings, validator, Error},
	Database,
};

pub async fn create_managed_database_in_organisation(
	connection: &mut <Database as sqlx::Database>::Connection,
	name: &str,
	db_name: &str,
	engine: &ManagedDatabaseEngine,
	version: Option<&str>,
	num_nodes: Option<u64>,
	database_plan: &ManagedDatabasePlan,
	region: &str,
	organisation_id: &[u8],
	config: &Settings,
) -> Result<Uuid, Error> {
	if !validator::is_database_name_valid(db_name) {
		log::trace!("Database name is invalid. Rejecting create request");
		return Err(Error::empty()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string()));
	}

	let (provider, region) = region
		.split_once('-')
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	log::trace!("generating new resource");
	let database_uuid = db::generate_new_resource_id(connection).await?;
	let database_id = database_uuid.as_bytes();

	let version = match engine {
		ManagedDatabaseEngine::Postgres => version.unwrap_or("12"),
		ManagedDatabaseEngine::Mysql => version.unwrap_or("8"),
	};
	let num_nodes = num_nodes.unwrap_or(1);

	db::create_resource(
		connection,
		database_id,
		&format!("{}-database-{}", provider, hex::encode(database_id)),
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(rbac::resource_types::MANAGED_DATABASE)
			.unwrap(),
		organisation_id,
		get_current_time_millis(),
	)
	.await?;

	log::trace!("creating entry for newly created managed database");
	db::create_managed_database(
		connection,
		database_id,
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
		organisation_id,
		None,
	)
	.await?;
	log::trace!("resource generation complete");

	match provider.parse() {
		Ok(CloudPlatform::DigitalOcean) => {
			digitalocean::create_managed_database_cluster(
				connection,
				database_id,
				db_name,
				engine,
				version,
				num_nodes,
				database_plan,
				region,
				config,
			)
			.await?;
		}
		Ok(CloudPlatform::Aws) => {
			aws::create_managed_database_cluster(
				connection,
				database_id,
				db_name,
				engine,
				version,
				num_nodes,
				database_plan,
				region,
				config,
			)
			.await?;
		}
		_ => {
			return Err(Error::empty()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string()));
		}
	}

	Ok(database_uuid)
}

pub async fn delete_managed_database(
	connection: &mut <Database as sqlx::Database>::Connection,
	database_id: &[u8],
	config: &Settings,
) -> Result<(), Error> {
	let database = db::get_managed_database_by_id(connection, database_id)
		.await?
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let (provider, region) = database
		.region
		.split_once('-')
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	match provider.parse() {
		Ok(CloudPlatform::DigitalOcean) => {
			log::trace!("deleting the database from digitalocean");
			if let Some(digitalocean_db_id) = database.digitalocean_db_id {
				digitalocean::delete_database(&digitalocean_db_id, config)
					.await?;
			}
		}
		Ok(CloudPlatform::Aws) => {
			log::trace!("deleting the deployment from aws");
			aws::delete_database(database_id, region).await?;
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
		&format!(
			"patr-deleted: {}-{}",
			database.name,
			hex::encode(database.id)
		),
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
	database_id: &[u8],
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
	database_id: &[u8],
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
