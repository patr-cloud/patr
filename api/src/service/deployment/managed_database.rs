use std::collections::HashMap;

use eve_rs::AsError;
use reqwest::Client;

use crate::{
	db,
	error,
	models::{
		deployment::cloud_providers::digitalocean::{
			DatabaseConfig,
			DatabaseResponse,
		},
		rbac,
	},
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
) -> Result<(), Error> {
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
	Ok(())
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
