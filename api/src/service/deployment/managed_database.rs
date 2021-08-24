use std::{collections::HashMap, ops::DerefMut, time::Duration};

use eve_rs::AsError;
use reqwest::Client;
use tokio::{
	task,
	time::{self, Instant},
};

use crate::{
	db,
	error,
	models::{
		db_mapping::{CloudPlatform, ManagedDatabaseStatus},
		deployment::cloud_providers::digitalocean::{
			DatabaseConfig,
			DatabaseResponse,
		},
		rbac,
	},
	service::{self},
	utils::{get_current_time_millis, settings::Settings, Error},
	Database,
};

pub async fn create_new_database_cluster(
	settings: Settings,
	name: &str,
	version: Option<&str>,
	engine: &str,
	num_nodes: Option<u64>,
	region: &str,
	organisation_id: &[u8],
	cloud_platform: CloudPlatform,
) -> Result<(), Error> {
	let name = name.to_string();
	let version = version.map(|v| v.to_string());
	let engine = engine.to_string();
	let region = region.to_string();
	let organisation_id = organisation_id.to_vec();
	task::spawn(async move {
		let result = match cloud_platform {
			CloudPlatform::DigitalOcean => {
				create_database_on_digitalocean(
					settings,
					name,
					version,
					engine,
					num_nodes,
					region,
					organisation_id,
				)
				.await
			}
			CloudPlatform::Aws => todo!(),
		};

		if let Err(error) = result {
			log::error!("Error while creating database, {}", error.get_error());
		}
	});

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
) -> Result<(), Error> {
	log::trace!("creating a digital ocean managed database");
	let app = service::get_app();
	let client = Client::new();

	let organisation_id = hex::encode(&organisation_id);
	log::trace!("parsing region");
	let region = parse_region(&region)?;

	let num_nodes = num_nodes.unwrap_or(1);

	let db_name = format!("{}-{}", organisation_id, name);
	log::trace!("sending the create db cluster request to digital ocean");
	let database_cluster = client
		.post("https://api.digitalocean.com/v2/databases")
		.bearer_auth(&settings.digital_ocean_api_key)
		.json(&DatabaseConfig {
			name: db_name, // should be unique
			engine,
			version,
			num_nodes,
			size: "db-s-1vcpu-1gb".to_string(),
			region: region.to_string(),
		})
		.send()
		.await?
		.json::<DatabaseResponse>()
		.await?;
	log::trace!("database created");
	log::trace!("generating new resource");
	let resource_id =
		db::generate_new_resource_id(app.database.acquire().await?.deref_mut())
			.await?;
	let resource_id = resource_id.as_bytes();

	let organisation_id = hex::decode(&organisation_id).unwrap();

	db::create_resource(
		app.database.acquire().await?.deref_mut(),
		resource_id,
		&format!("do-database-{}", name),
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
		&database_cluster.database.name,
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

	log::trace!("waiting for databse to be online");
	wait_for_database_cluster_to_be_online(
		app.database.acquire().await?.deref_mut(),
		settings,
		resource_id,
		&database_cluster.database.id,
		client,
	)
	.await?;
	log::trace!("database online");

	log::trace!("updating the entry after the database is online");
	db::update_managed_database(
		app.database.acquire().await?.deref_mut(),
		&database_cluster.database.name,
		&database_cluster.database.id,
		&database_cluster.database.engine,
		&database_cluster.database.version,
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

// TODO: refactor this
fn parse_region(region: &str) -> Result<&str, Error> {
	let mut data_centers = HashMap::new();

	data_centers.insert("do-nyc", "nyc3");
	data_centers.insert("do-ams", "ams3");
	data_centers.insert("do-sfo", "sfo3");
	data_centers.insert("do-sgp", "sgp1");
	data_centers.insert("do-lon", "lon1");
	data_centers.insert("do-fra", "fra1");
	data_centers.insert("do-tor", "tor1");
	data_centers.insert("do-blr", "blr1");
	data_centers.insert("aws-us-east-1", "us-east-1");
	data_centers.insert("aws-us-east-2", "us-east-2");
	data_centers.insert("aws-us-west-2", "us-west-2");
	data_centers.insert("aws-ap-south-1", "ap-south-1");
	data_centers.insert("aws-ap-northeast-2", "ap-northeast-2");
	data_centers.insert("aws-ap-southeast-1", "ap-southeast-1");
	data_centers.insert("aws-ap-southeast-2", "ap-southeast-2");
	data_centers.insert("aws-ap-northeast-1", "ap-northeast-1");
	data_centers.insert("aws-ca-central-1", "ca-central-1");
	data_centers.insert("aws-eu-central-1", "eu-central-1");
	data_centers.insert("aws-eu-west-1", "eu-west-1");
	data_centers.insert("aws-eu-west-2", "eu-west-2");
	data_centers.insert("aws-eu-west-3", "eu-west-3");
	data_centers.insert("aws-eu-north-1", "eu-north-1");

	let region = data_centers.get(region);

	if let Some(region) = region {
		Ok(region)
	} else {
		Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())
	}
}

async fn wait_for_database_cluster_to_be_online(
	connection: &mut <Database as sqlx::Database>::Connection,
	settings: Settings,
	database_id: &[u8],
	cloud_db_id: &str,
	client: Client,
) -> Result<(), Error> {
	let start = Instant::now();
	loop {
		let database_status =
			get_cluster_from_digital_ocean(&client, &settings, cloud_db_id)
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
			delete_database(
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

pub async fn delete_database(
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
