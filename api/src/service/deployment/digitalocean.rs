use std::{process::Stdio, str, time::Duration};

use api_models::{
	models::workspace::infrastructure::deployment::DeploymentStatus,
	utils::Uuid,
};
use eve_rs::AsError;
use reqwest::Client;
use tokio::{process::Command, task, time};

use crate::{
	db,
	error,
	models::{
		db_mapping::{
			ManagedDatabaseEngine,
			ManagedDatabasePlan,
			ManagedDatabaseStatus,
		},
		deployment::cloud_providers::digitalocean::{
			Auth,
			DatabaseConfig,
			DatabaseResponse,
			Db,
		},
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
	database_plan: &ManagedDatabasePlan,
	region: &str,
	config: &Settings,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();
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
			size: database_plan.as_do_plan()?,
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

	task::spawn(async move {
		let result = update_database_cluster_credentials(
			database_id.clone(),
			db_name,
			database_cluster.database.id,
			request_id,
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
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();
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

pub(super) async fn delete_image_from_digitalocean_registry(
	deployment_id: &Uuid,
	config: &Settings,
) -> Result<(), Error> {
	let client = Client::new();

	let container_status = client
		.delete(format!(
			"https://api.digitalocean.com/v2/registry/{}/repositories/{}/tags/latest",
			config.digitalocean.registry,
			deployment_id,
		))
		.bearer_auth(&config.digitalocean.api_key)
		.send()
		.await?
		.status();

	if container_status.is_server_error() || container_status.is_client_error()
	{
		return Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string());
	}

	Ok(())
}

pub async fn push_to_docr(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
	full_image_name: &str,
	client: Client,
	config: &Settings,
) -> Result<String, Error> {
	// Fetch the image from patr registry
	// upload the image to DOCR
	// Update kubernetes

	let request_id = Uuid::new_v4();
	log::trace!("request_id: {} - Pulling image from registry", request_id);
	service::pull_image_from_registry(full_image_name, config).await?;
	// log::trace!("request_id: {} - Image pulled", request_id);

	// new name for the docker image
	let new_repo_name = format!(
		"registry.digitalocean.com/{}/{}",
		config.digitalocean.registry, deployment_id,
	);
	log::trace!("request_id: {} - Pushing to {}", request_id, new_repo_name);

	// rename the docker image with the digital ocean registry url
	service::tag_docker_image(full_image_name, &new_repo_name).await?;
	log::trace!("request_id: {} - Image tagged", request_id);

	// Get login details from digital ocean registry and decode from
	// base 64 to binary
	let auth_token =
		base64::decode(get_registry_auth_token(config, &client).await?)?;
	log::trace!("request_id: {} - Got auth token", request_id);

	// Convert auth token from binary to utf8
	let auth_token = str::from_utf8(&auth_token)?;
	log::trace!("request_id: {} - Decoded auth token", request_id);

	// get username and password from the auth token
	let (username, password) = auth_token
		.split_once(":")
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	// Login into the registry
	let output = Command::new("docker")
		.arg("login")
		.arg("-u")
		.arg(username)
		.arg("-p")
		.arg(password)
		.arg("registry.digitalocean.com")
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()?
		.wait()
		.await?;
	log::trace!("request_id: {} - Logged into DO registry", request_id);

	if !output.success() {
		return Err(Error::empty()
			.status(500)
			.body(error!(SERVER_ERROR).to_string()));
	}
	log::trace!("request_id: {} - Login was success", request_id);

	// if the loggin in is successful the push the docker image to
	// registry
	let push_status = Command::new("docker")
		.arg("push")
		.arg(&new_repo_name)
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()?
		.wait()
		.await?;
	log::trace!(
		"request_id: {} - Pushing to DO to {}",
		request_id,
		new_repo_name,
	);

	if !push_status.success() {
		return Err(Error::empty()
			.status(500)
			.body(error!(SERVER_ERROR).to_string()));
	}

	db::update_deployment_status(
		connection,
		deployment_id,
		&DeploymentStatus::Pushed,
	)
	.await?;

	log::trace!("request_id: {} - Pushed to DO", request_id);
	log::trace!("Deleting image tagged with registry.digitalocean.com");
	let delete_result = super::delete_docker_image(&new_repo_name).await;
	if let Err(delete_result) = delete_result {
		log::error!(
			"Failed to delete the image: {}, Error: {}",
			new_repo_name,
			delete_result.get_error()
		);
	}

	log::trace!("deleting the pulled image");
	let delete_result = super::delete_docker_image(full_image_name).await;
	if let Err(delete_result) = delete_result {
		log::error!(
			"Failed to delete the image: {}, Error: {}",
			full_image_name,
			delete_result.get_error()
		);
	}
	log::trace!("Docker image deleted");
	Ok(new_repo_name)
}

async fn update_database_cluster_credentials(
	database_id: Uuid,
	db_name: String,
	digitalocean_db_id: String,
	request_id: Uuid,
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

async fn get_registry_auth_token(
	config: &Settings,
	client: &Client,
) -> Result<String, Error> {
	let registry = client
		.get("https://api.digitalocean.com/v2/registry/docker-credentials?read_write=true?expiry_seconds=86400")
		.bearer_auth(&config.digitalocean.api_key)
		.send()
		.await?
		.json::<Auth>()
		.await?;

	Ok(registry.auths.registry.auth)
}
