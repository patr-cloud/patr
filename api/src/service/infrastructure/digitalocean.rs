use std::{str, time::Duration};

use api_models::utils::Uuid;
use eve_rs::AsError;
use reqwest::Client;
use tokio::{task, time};

use crate::{
	db::{
		self,
		ManagedDatabaseEngine,
		ManagedDatabasePlan,
		ManagedDatabaseStatus,
	},
	error,
	models::deployment::cloud_providers::digitalocean::{
		DatabaseConfig,
		DatabaseResponse,
		Db,
	},
	service,
	utils::{get_current_time, settings::Settings, Error},
	Database,
};

pub(super) async fn create_managed_database_cluster(
	connection: &mut <Database as sqlx::Database>::Connection,
	database_id: &Uuid,
	db_name: &str,
	engine: &ManagedDatabaseEngine,
	version: &str,
	num_nodes: u64,
	_database_plan: &ManagedDatabasePlan,
	region: &str,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!("Creating a managed database on digitalocean with id: {} and db_name: {} on DigitalOcean App platform with request_id: {}",
		database_id,
		db_name,
		request_id
	);
	let client = Client::new();

	let do_db_name = format!("database-{}", get_current_time().as_millis());

	let db_engine = match engine {
		ManagedDatabaseEngine::Postgres => "pg",
		ManagedDatabaseEngine::Mysql => "mysql",
	};

	let region = match region {
		"nyc" => "nyc1",
		"ams" => "ams3",
		"sfo" => "sfo3",
		"sgp" => "sgp1",
		"lon" => "lon1",
		"fra" => "fra1",
		"tor" => "tor1",
		"blr" => "blr1",
		"any" => "blr1",
		_ => {
			return Err(Error::empty()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string()))
		}
	};

	log::trace!("request_id: {} - sending the create db cluster request to digital ocean", request_id);
	let database_cluster = client
		.post("https://api.digitalocean.com/v2/databases")
		.bearer_auth(&config.digitalocean.api_key)
		.json(&DatabaseConfig {
			name: do_db_name, // should be unique
			engine: db_engine.to_string(),
			version: Some(version.to_string()),
			num_nodes,
			size: "db-s-1vcpu-1gb".to_string(),
			region: region.to_string(),
		})
		.send()
		.await?
		.json::<DatabaseResponse>()
		.await?;
	log::trace!("request_id: {} - database created", request_id);

	db::update_digitalocean_db_id_for_database(
		connection,
		database_id,
		&database_cluster.database.id,
	)
	.await?;

	let database_id = database_id.clone();
	let db_name = db_name.to_string();

	let request_id = request_id.clone();
	task::spawn(async move {
		let result = update_database_cluster_credentials(
			database_id.clone(),
			db_name,
			database_cluster.database.id,
			&request_id,
		)
		.await;

		if let Err(error) = result {
			let _ = super::update_managed_database_status(
				&database_id,
				&ManagedDatabaseStatus::Errored,
			)
			.await;
			log::error!(
				"Error while creating managed database, {}",
				error.get_error()
			);
		}
	});

	Ok(())
}

pub(super) async fn delete_database(
	digitalocean_db_id: &str,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!("Deleting managed database on DigitalOcean with digital_ocean_id: {} and request_id: {}",
		digitalocean_db_id,
		request_id,
	);
	let client = Client::new();

	let database_status = client
		.delete(format!(
			"https://api.digitalocean.com/v2/databases/{}",
			digitalocean_db_id
		))
		.bearer_auth(&config.digitalocean.api_key)
		.send()
		.await?
		.status();

	if database_status.is_client_error() || database_status.is_server_error() {
		return Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;
	}
	log::trace!("request_id: {} - database deletion successfull", request_id);
	Ok(())
}

async fn update_database_cluster_credentials(
	database_id: Uuid,
	db_name: String,
	digitalocean_db_id: String,
	request_id: &Uuid,
) -> Result<(), Error> {
	let client = Client::new();
	let settings = service::get_settings();

	// Wait for the database to be online
	log::trace!(
		"request_id: {} - waiting for database to be online",
		request_id
	);
	loop {
		let database_status = client
			.get(format!(
				"https://api.digitalocean.com/v2/databases/{}",
				digitalocean_db_id
			))
			.bearer_auth(&settings.digitalocean.api_key)
			.send()
			.await?
			.json::<DatabaseResponse>()
			.await?;

		if database_status.database.status == "online" {
			super::update_managed_database_credentials_for_database(
				&database_id,
				&database_status.database.connection.host,
				database_status.database.connection.port as i32,
				&database_status.database.connection.user,
				&database_status.database.connection.password,
			)
			.await?;
			super::update_managed_database_status(
				&database_id,
				&ManagedDatabaseStatus::Running,
			)
			.await?;
			break;
		}

		time::sleep(Duration::from_millis(1000)).await;
	}
	log::trace!("request_id: {} - database online", request_id);

	log::trace!(
		"request_id: {} - creating a new database inside cluster",
		request_id
	);
	let new_db_status = client
		.post(format!(
			"https://api.digitalocean.com/v2/databases/{}/dbs",
			digitalocean_db_id
		))
		.bearer_auth(&settings.digitalocean.api_key)
		.json(&Db { name: db_name })
		.send()
		.await?
		.status();

	if new_db_status.is_client_error() || new_db_status.is_server_error() {
		return Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;
	}

	log::trace!(
		"request_id: {} - updating to the db status to running",
		request_id
	);
	// wait for database to start
	super::update_managed_database_status(
		&database_id,
		&ManagedDatabaseStatus::Running,
	)
	.await?;
	log::trace!("request_id: {} - database successfully updated", request_id);

	Ok(())
}
