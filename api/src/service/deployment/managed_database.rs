use std::{ops::DerefMut, time::Duration};

use eve_rs::AsError;
use lightsail::model::RelationalDatabase;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use reqwest::Client;
use serde_json::json;
use tokio::{
	task,
	time::{self, Instant},
};

use crate::{
	db,
	error,
	models::{
		db_mapping::{
			CloudPlatform,
			DatabasePlan,
			Engine,
			ManagedDatabaseStatus,
		},
		deployment::cloud_providers::digitalocean::{
			DatabaseConfig,
			DatabaseInfo,
			DatabaseNamewrapper,
			DatabaseResponse,
		},
		rbac,
	},
	service::{self, deployment::aws},
	utils::{
		get_current_time,
		get_current_time_millis,
		settings::Settings,
		validator,
		Error,
	},
	Database,
};

pub async fn create_database_cluster(
	settings: Settings,
	name: &str,
	version: Option<&str>,
	engine: &str,
	num_nodes: Option<u64>,
	region: &str,
	organisation_id: &[u8],
	database_plan: DatabasePlan,
) -> Result<(), Error> {
	let name = name.to_string();
	let version = version.map(|v| v.to_string());
	let engine = engine.to_string();
	let organisation_id = organisation_id.to_vec();
	let (provider, region) = region
		.split_once('-')
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;
	let region = region.to_string();

	match provider.parse() {
		Ok(CloudPlatform::DigitalOcean) => {
			task::spawn(async move {
				let result = create_database_on_digitalocean(
					settings,
					name,
					version,
					engine,
					num_nodes,
					region,
					organisation_id,
					database_plan,
				)
				.await;

				if let Err(error) = result {
					log::error!(
						"Error while creating database, {}",
						error.get_error()
					);
				}
			});
		}
		Ok(CloudPlatform::Aws) => {
			task::spawn(async move {
				let result = create_database_on_aws(
					name,
					version,
					engine,
					region,
					organisation_id,
					database_plan,
				)
				.await;

				if let Err(error) = result {
					log::error!(
						"Error while creating database, {}",
						error.get_error()
					);
				}
			});
		}
		_ => {
			return Err(Error::empty()
				.status(500)
				.body(error!(SERVER_ERROR).to_string()));
		}
	}

	Ok(())
}

async fn create_database_on_digitalocean(
	settings: Settings,
	name: String,
	version: Option<String>,
	engine: String,
	num_nodes: Option<u64>,
	region: String,
	organisation_id: Vec<u8>,
	database_plan: DatabasePlan,
) -> Result<(), Error> {
	log::trace!("creating a digital ocean managed database");
	let app = service::get_app();
	let engine = engine.parse::<Engine>()?;

	log::trace!("checking if the database name is valid or not");
	if !validator::is_database_name_valid(&name) {
		log::trace!("database name invalid");
		Error::as_result()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string())?;
	}

	let version = if engine == Engine::Postgres {
		version.unwrap_or_else(|| "12".to_string())
	} else {
		version.unwrap_or_else(|| "8".to_string())
	};

	let client = Client::new();

	let num_nodes = num_nodes.unwrap_or(1);

	log::trace!("generating new resource");
	let resource_id =
		db::generate_new_resource_id(app.database.acquire().await?.deref_mut())
			.await?;
	let resource_id = resource_id.as_bytes();

	let db_name = format!("database-{}", get_current_time().as_millis());

	let db_engine = if engine == Engine::Postgres {
		"pg"
	} else {
		"mysql"
	};

	let region = match region.as_str() {
		"nyc" => "nyc1",
		"ams" => "ams3",
		"sfo" => "sfo3",
		"sgp" => "sgp1",
		"lon" => "lon1",
		"fra" => "fra1",
		"tor" => "tor1",
		"blr" => "blr1",
		_ => "blr1",
	};

	log::trace!("sending the create db cluster request to digital ocean");
	let database_cluster = client
		.post("https://api.digitalocean.com/v2/databases")
		.bearer_auth(&settings.digital_ocean_api_key)
		.json(&DatabaseConfig {
			name: db_name, // should be unique
			engine: db_engine.to_string(),
			version: Some(version.clone()),
			num_nodes,
			size: database_plan.to_string(),
			region: region.to_string(),
		})
		.send()
		.await?
		.json::<DatabaseResponse>()
		.await?;
	log::trace!("database created");

	db::create_resource(
		app.database.acquire().await?.deref_mut(),
		resource_id,
		&format!("do-database-{}", hex::encode(resource_id)),
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(rbac::resource_types::MANAGED_DATABASE)
			.unwrap(),
		&organisation_id,
		get_current_time_millis(),
	)
	.await?;
	log::trace!("resource generation complete");

	log::trace!("creating entry for newly created managed database");
	db::create_managed_database(
		app.database.acquire().await?.deref_mut(),
		resource_id,
		&name,
		CloudPlatform::DigitalOcean,
		&organisation_id,
	)
	.await?;

	log::trace!("updating to the db status to creating");
	// wait for database to start
	db::update_managed_database_status(
		app.database.acquire().await?.deref_mut(),
		resource_id,
		&ManagedDatabaseStatus::Creating,
	)
	.await?;

	log::trace!("waiting for database to be online");
	wait_for_digitalocean_database_cluster_to_be_online(
		app.database.acquire().await?.deref_mut(),
		&settings,
		resource_id,
		&database_cluster.database.id,
		&client,
	)
	.await?;
	log::trace!("database online");

	log::trace!("creating a new database inside cluster");
	let new_database = client
		.post(format!(
			"https://api.digitalocean.com/v2/databases/{}/dbs",
			database_cluster.database.id
		))
		.bearer_auth(&settings.digital_ocean_api_key)
		.json(&json!({ "name": name }))
		.send()
		.await?
		.json::<DatabaseNamewrapper>()
		.await?;

	log::trace!("updating the entry after the database is online");
	db::update_managed_database(
		app.database.acquire().await?.deref_mut(),
		&new_database.db.name,
		&database_cluster.database.id,
		engine,
		&version,
		database_cluster.database.num_nodes as i32,
		&database_cluster.database.size,
		&database_cluster.database.region,
		ManagedDatabaseStatus::Running,
		&database_cluster.database.connection.host,
		database_cluster.database.connection.port as i32,
		&database_cluster.database.connection.user,
		&database_cluster.database.connection.password,
		&organisation_id,
	)
	.await?;
	db::update_digital_ocean_db_id_for_database(
		app.database.acquire().await?.deref_mut(),
		&database_cluster.database.name,
		resource_id,
	)
	.await?;
	log::trace!("database successfully updated");

	Ok(())
}

pub async fn get_all_database_clusters_for_organisation(
	connection: &mut <Database as sqlx::Database>::Connection,
	settings: Settings,
	organisation_id: &[u8],
) -> Result<Vec<DatabaseResponse>, Error> {
	let clusters = db::get_all_database_clusters_for_organisation(
		connection,
		organisation_id,
	)
	.await?;

	let mut cluster_list = Vec::new();
	let client = Client::new();
	for cluster in clusters {
		if let Some(cloud_database_id) = cluster.cloud_database_id {
			let managed_db_cluster = get_cluster_from_digital_ocean(
				&client,
				&settings,
				&cloud_database_id,
			)
			.await?;
			cluster_list.push(managed_db_cluster);
		}
	}

	Ok(cluster_list)
}

async fn wait_for_digitalocean_database_cluster_to_be_online(
	connection: &mut <Database as sqlx::Database>::Connection,
	settings: &Settings,
	database_id: &[u8],
	cloud_db_id: &str,
	client: &Client,
) -> Result<(), Error> {
	let start = Instant::now();
	loop {
		let database_status =
			get_cluster_from_digital_ocean(client, settings, cloud_db_id)
				.await?;

		if database_status.database.status == *"online" {
			db::update_managed_database_status(
				connection,
				database_id,
				&ManagedDatabaseStatus::Running,
			)
			.await?;
			break;
		}

		if start.elapsed() > Duration::from_secs(900) {
			db::update_managed_database_status(
				connection,
				database_id,
				&ManagedDatabaseStatus::Errored,
			)
			.await?;
			let settings = settings.clone();
			let database_id = database_id.to_vec();
			let cloud_db_id = cloud_db_id.to_string();
			let client = client.clone();
			task::spawn(async move {
				let result = wait_and_delete_the_running_database(
					&settings,
					&database_id,
					&cloud_db_id,
					&client,
				)
				.await;
				if let Err(error) = result {
					log::info!(
						"Error while creating databse: {}",
						error.get_error()
					);
				}
			});
			return Error::as_result()
				.status(500)
				.body(error!(SERVER_ERROR).to_string())?;
		}
		time::sleep(Duration::from_millis(1000)).await;
	}
	Ok(())
}

async fn wait_and_delete_the_running_database(
	settings: &Settings,
	database_id: &[u8],
	cloud_db_id: &str,
	client: &Client,
) -> Result<(), Error> {
	let app = service::get_app();
	loop {
		let database_status =
			get_cluster_from_digital_ocean(client, settings, cloud_db_id)
				.await?;

		if database_status.database.status == *"online" {
			delete_managed_database(
				app.database.acquire().await?.deref_mut(),
				settings,
				database_id,
				cloud_db_id,
				client,
			)
			.await?;
			break;
		}
	}
	Ok(())
}

async fn get_cluster_from_digital_ocean(
	client: &Client,
	settings: &Settings,
	cloud_db_id: &str,
) -> Result<DatabaseResponse, Error> {
	let database_status = client
		.get(format!(
			"https://api.digitalocean.com/v2/databases/{}",
			cloud_db_id
		))
		.bearer_auth(&settings.digital_ocean_api_key)
		.send()
		.await?
		.json::<DatabaseResponse>()
		.await?;
	Ok(database_status)
}

pub async fn get_managed_database_info_for_organisation(
	connection: &mut <Database as sqlx::Database>::Connection,
	settings: &Settings,
	name: &str,
	organisation_id: &[u8],
) -> Result<(DatabaseInfo, ManagedDatabaseStatus), Error> {
	let cloud_db = db::get_managed_database_by_name_and_org_id(
		connection,
		name,
		organisation_id,
	)
	.await?
	.status(404)
	.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let status = cloud_db.status;
	let cloud_db_id = cloud_db
		.cloud_database_id
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let client = Client::new();
	let database_info =
		get_cluster_from_digital_ocean(&client, settings, &cloud_db_id).await?;

	Ok((database_info.database, status))
}

pub async fn delete_managed_database(
	connection: &mut <Database as sqlx::Database>::Connection,
	settings: &Settings,
	database_id: &[u8],
	cloud_db_id: &str,
	client: &Client,
) -> Result<(), Error> {
	let database_status = client
		.delete(format!(
			"https://api.digitalocean.com/v2/databases/{}",
			cloud_db_id
		))
		.bearer_auth(&settings.digital_ocean_api_key)
		.send()
		.await?
		.status();

	if database_status.is_client_error() || database_status.is_server_error() {
		return Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;
	}

	db::update_managed_database_status(
		connection,
		database_id,
		&ManagedDatabaseStatus::Deleted,
	)
	.await?;

	Ok(())
}

async fn create_database_on_aws(
	name: String,
	version: Option<String>,
	engine: String,
	region: String,
	organisation_id: Vec<u8>,
	database_plan: DatabasePlan,
) -> Result<(), Error> {
	log::trace!("creating a aws managed database");
	let app = service::get_app();
	let engine = engine.parse::<Engine>()?;

	log::trace!("checking if the database name is valid or not");
	if !validator::is_database_name_valid(&name) {
		log::trace!("database name invalid");
		Error::as_result()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string())?;
	}

	let version = if engine == Engine::Postgres {
		version.unwrap_or_else(|| "12".to_string())
	} else {
		version.unwrap_or_else(|| "8".to_string())
	};
	let master_username = format!("user_{}", name);
	let client = aws::get_lightsail_client(&region);

	log::trace!("generating new resource");
	let resource_id =
		db::generate_new_resource_id(app.database.acquire().await?.deref_mut())
			.await?;
	let resource_id = resource_id.as_bytes();

	db::create_resource(
		app.database.acquire().await?.deref_mut(),
		resource_id,
		&format!("aws-database-{}", hex::encode(resource_id)),
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(rbac::resource_types::MANAGED_DATABASE)
			.unwrap(),
		&organisation_id,
		get_current_time_millis(),
	)
	.await?;
	log::trace!("resource generation complete");

	let password: String = thread_rng()
		.sample_iter(&Alphanumeric)
		.take(8)
		.map(char::from)
		.collect();

	log::trace!("sending the create db cluster request to aws");
	client
		.create_relational_database()
		.master_database_name(&name)
		.master_username(&master_username)
		.master_user_password(&password)
		.publicly_accessible(true)
		.relational_database_blueprint_id(format!("{}_{}", engine, version))
		.relational_database_bundle_id(database_plan.to_string())
		.relational_database_name(hex::encode(&resource_id))
		.send()
		.await?;
	log::trace!("database created");

	log::trace!("creating entry for newly created managed database");
	db::create_managed_database(
		app.database.acquire().await?.deref_mut(),
		resource_id,
		&name,
		CloudPlatform::Aws,
		&organisation_id,
	)
	.await?;

	log::trace!("updating to the db status to creating");
	// wait for database to start
	db::update_managed_database_status(
		app.database.acquire().await?.deref_mut(),
		resource_id,
		&ManagedDatabaseStatus::Creating,
	)
	.await?;

	// we can get id from aws of the resource but right now it is not used
	// anywhere let database_id = database
	// 	.operations
	// 	.map(|operation| operation.into_iter().next())
	// 	.flatten()
	// 	.map(|op| op.id)
	// 	.flatten()
	// 	.status(500)
	// 	.body(error!(SERVER_ERROR).to_string())?;

	log::trace!("waiting for database to be online");
	let database_info = wait_for_aws_database_cluster_to_be_online(
		app.database.acquire().await?.deref_mut(),
		resource_id,
		client,
	)
	.await?;
	log::trace!("database online");

	let address = database_info
		.master_endpoint
		.clone()
		.map(|address| address.address)
		.flatten()
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	let port = database_info
		.master_endpoint
		.map(|port| port.port)
		.flatten()
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;
	log::trace!("updating the entry after the database is online");
	db::update_managed_database(
		app.database.acquire().await?.deref_mut(),
		&name,
		&hex::encode(resource_id),
		engine,
		&version,
		1,
		&database_plan.to_string(),
		&region,
		ManagedDatabaseStatus::Running,
		&address,
		port as i32,
		&master_username,
		&password,
		&organisation_id,
	)
	.await?;
	log::trace!("database successfully updated");

	Ok(())
}

async fn wait_for_aws_database_cluster_to_be_online(
	connection: &mut <Database as sqlx::Database>::Connection,
	resource_id: &[u8],
	client: lightsail::Client,
) -> Result<RelationalDatabase, Error> {
	loop {
		let database_info = client
			.get_relational_database()
			.relational_database_name(hex::encode(resource_id))
			.send()
			.await?;

		let database_state = database_info
			.clone()
			.relational_database
			.map(|rdbms| rdbms.state)
			.flatten()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;

		if database_state == "available" {
			db::update_managed_database_status(
				connection,
				resource_id,
				&ManagedDatabaseStatus::Running,
			)
			.await?;

			let db_info = database_info
				.relational_database
				.status(500)
				.body(error!(SERVER_ERROR).to_string())?;

			return Ok(db_info);
		} else if database_state != "creating" &&
			database_state != "configuring-log-exports" &&
			database_state != "backing-up"
		{
			break;
		}
		time::sleep(Duration::from_millis(1000)).await;
	}

	db::update_managed_database_status(
		connection,
		resource_id,
		&ManagedDatabaseStatus::Errored,
	)
	.await?;

	Error::as_result()
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?
}
