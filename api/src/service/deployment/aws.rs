use std::time::Duration;

use eve_rs::AsError;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use tokio::{task, time};
use uuid::Uuid;

use crate::{
	error,
	models::db_mapping::{
		ManagedDatabaseEngine,
		ManagedDatabasePlan,
		ManagedDatabaseStatus,
	},
	service::deployment::managed_database,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn create_managed_database_cluster(
	_connection: &mut <Database as sqlx::Database>::Connection,
	database_id: &[u8],
	db_name: &str,
	engine: &ManagedDatabaseEngine,
	version: &str,
	_num_nodes: u64,
	database_plan: &ManagedDatabasePlan,
	region: &str,
	_config: &Settings,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();
	log::trace!("Creating a managed database on aws lightsail with id: {} and db_name: {} with request_id: {}",
		hex::encode(&database_id),
		db_name,
		request_id
	);
	let client = get_lightsail_client(region);

	let username = "patr_admin".to_string();
	let password = thread_rng()
		.sample_iter(&Alphanumeric)
		.take(8)
		.map(char::from)
		.collect::<String>();

	log::trace!(
		"request_id: {} - sending the create db cluster request to aws",
		request_id
	);
	client
		.create_relational_database()
		.master_database_name(db_name)
		.master_username(&username)
		.master_user_password(&password)
		.publicly_accessible(true)
		.relational_database_blueprint_id(format!(
			"{}_{}",
			engine,
			match version {
				"8" => "8_0",
				value => value,
			}
		))
		.relational_database_bundle_id(database_plan.as_aws_plan()?)
		.relational_database_name(hex::encode(database_id))
		.send()
		.await?;
	log::trace!("request_id: {} - database created", request_id);

	let database_id = database_id.to_vec();
	let region = region.to_string();

	task::spawn(async move {
		let result = update_database_cluster_credentials(
			database_id.clone(),
			region,
			username,
			password,
			request_id,
		)
		.await;

		if let Err(error) = result {
			let _ = managed_database::update_managed_database_status(
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
	database_id: &[u8],
	region: &str,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();
	log::trace!("Deleting managed database on Awl lightsail with digital_ocean_id: {} and request_id: {}",
		hex::encode(database_id),
		request_id,
	);

	log::trace!("request_id: {} - getting lightsail client", request_id);
	let client = get_lightsail_client(region);

	log::trace!(
		"request_id: {} - getting database info from lightsail",
		request_id
	);
	let database_cluster = client
		.get_relational_database()
		.relational_database_name(hex::encode(database_id))
		.send()
		.await;

	if database_cluster.is_err() {
		return Ok(());
	}

	log::trace!(
		"request_id: {} - deleting database from lightsail",
		request_id
	);
	client
		.delete_relational_database()
		.relational_database_name(hex::encode(database_id))
		.send()
		.await?;

	Ok(())
}

pub(super) async fn get_app_default_url(
	deployment_id: &str,
	region: &str,
) -> Result<Option<String>, Error> {
	let client = get_lightsail_client(region);
	let default_url = client
		.get_container_services()
		.service_name(deployment_id)
		.send()
		.await
		.ok()
		.map(|services| {
			services
				.container_services
				.map(|services| services.into_iter().next())
				.flatten()
		})
		.flatten()
		.map(|service| service.url)
		.flatten()
		.map(|url| url.replace("https://", "").replace("/", ""));

	Ok(default_url)
}

async fn update_database_cluster_credentials(
	database_id: Vec<u8>,
	region: String,
	username: String,
	password: String,
	request_id: Uuid,
) -> Result<(), Error> {
	let client = get_lightsail_client(&region);

	log::trace!(
		"request_id: {} - getting database info from lightsail",
		request_id
	);
	let (host, port) = loop {
		let database = client
			.get_relational_database()
			.relational_database_name(hex::encode(&database_id))
			.send()
			.await?
			.relational_database
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;

		let database_state = database
			.state
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;

		log::trace!("request_id: {} - checking the database state", request_id);
		match database_state.as_str() {
			"available" => {
				// update credentials
				let (host, port) = database
					.master_endpoint
					.map(|endpoint| endpoint.address.zip(endpoint.port))
					.flatten()
					.status(500)
					.body(error!(SERVER_ERROR).to_string())?;
				break (host, port);
			}
			"creating" | "configuring-log-exports" | "backing-up" => {
				// Database still being created. Wait
				time::sleep(Duration::from_millis(1000)).await;
			}
			_ => {
				// Database is neither being created nor available. Consider it
				// to be Errored
				super::update_managed_database_status(
					&database_id,
					&ManagedDatabaseStatus::Errored,
				)
				.await?;

				return Err(Error::empty()
					.status(500)
					.body(error!(SERVER_ERROR).to_string()));
			}
		}
	};

	log::trace!(
		"request_id: {} updating managed database credentials",
		request_id
	);
	managed_database::update_managed_database_credentials_for_database(
		&database_id,
		&host,
		port,
		&username,
		&password,
	)
	.await?;

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

fn get_lightsail_client(region: &str) -> lightsail::Client {
	let deployment_region = lightsail::Region::new(region.to_string());
	let client_builder = lightsail::Config::builder()
		.region(Some(deployment_region))
		.build();
	lightsail::Client::from_conf(client_builder)
}
