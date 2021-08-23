use std::{collections::HashMap, ops::DerefMut, time::Duration};

use eve_rs::AsError;
use reqwest::Client;
use tokio::{task, time::Instant};

use crate::{
	db,
	error,
	models::{
		db_mapping::ManagedDatabaseStatus,
		deployment::cloud_providers::digitalocean::{
			DatabaseConfig,
			DatabaseInfo,
			DatabaseResponse,
		},
		rbac,
	},
	service,
	utils::{get_current_time_millis, settings::Settings, Error},
	Database,
};

pub async fn create_new_database_cluster(
	connection: &mut <Database as sqlx::Database>::Connection,
	settings: Settings,
	name: &str,
	version: Option<&str>,
	engine: &str,
	num_nodes: u64,
	region: &str,
	organisation_id: &[u8],
) -> Result<DatabaseInfo, Error> {
	let client = Client::new();

	let organisation_id = hex::encode(&organisation_id);
	let region = parse_region(region)?;

	let db_name = format!("{}-{}", organisation_id, name);

	let database_cluster = client
		.post("https://api.digitalocean.com/v2/databases")
		.bearer_auth(&settings.digital_ocean_api_key)
		.json(&DatabaseConfig {
			name: db_name, // should be unique
			engine: engine.to_string(),
			version: version.map(|v| v.to_string()),
			num_nodes,
			size: "db-s-1vcpu-1gb".to_string(),
			region: region.to_string(),
		})
		.send()
		.await?
		.json::<DatabaseResponse>()
		.await?;

	let resource_id = db::generate_new_resource_id(connection).await?;
	let resource_id = resource_id.as_bytes();

	let organisation_id = hex::decode(&organisation_id).unwrap();

	db::create_resource(
		connection,
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

	db::create_managed_database(
		connection,
		resource_id,
		&database_cluster.database.name,
		&database_cluster.database.id,
		"DigitalOcean",
		&organisation_id,
	)
	.await?;

	// wait for database to start
	db::update_managed_database_status(
		connection,
		resource_id,
		&ManagedDatabaseStatus::Creating,
	)
	.await?;

	wait_for_database_cluster_to_be_online(
		connection,
		settings,
		resource_id,
		&database_cluster.database.id,
		client,
	)
	.await?;

	Ok(database_cluster.database)
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
		if cluster.database_id.is_some() {
			let managed_db_cluster = get_cluster_from_digital_ocean(
				&client,
				&settings,
				&cluster.database_id.unwrap(),
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
			get_cluster_from_digital_ocean(&client, &settings, &cloud_db_id)
				.await?;

		if database_status.database.status == "online".to_string() {
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
						"Error with the deployment, {}",
						error.get_error()
					);
				}
			});
			return Error::as_result()
				.status(500)
				.body(error!(SERVER_ERROR).to_string())?;
		}
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
			get_cluster_from_digital_ocean(client, settings, &cloud_db_id)
				.await?;

		if database_status.database.status == "online".to_string() {
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
