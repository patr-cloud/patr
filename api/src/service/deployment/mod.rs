mod aws;
mod digitalocean;

use std::ops::DerefMut;

use cloudflare::{
	endpoints::{
		dns::{
			CreateDnsRecord,
			CreateDnsRecordParams,
			DnsContent,
			ListDnsRecords,
			ListDnsRecordsParams,
			UpdateDnsRecord,
			UpdateDnsRecordParams,
		},
		zone::{ListZones, ListZonesParams},
	},
	framework::{
		async_api::{ApiClient, Client as CloudflareClient},
		auth::Credentials,
		Environment,
		HttpApiClientConfig,
	},
};
use eve_rs::AsError;
use futures::StreamExt;
use shiplift::{Docker, PullOptions, RegistryAuth, TagOptions};
use tokio::task;
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
			CNameRecord,
			DeploymentMachineType,
			DeploymentStatus,
		},
		rbac,
		RegistryToken,
		RegistryTokenAccess,
	},
	service,
	utils::{
		get_current_time,
		get_current_time_millis,
		settings::Settings,
		validator,
		Error,
	},
	Database,
};

/// # Description
/// This function creates a deployment under an organisation account
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `organisation_id` -  an unsigned 8 bit integer array containing the id of
///   organisation
/// * `name` - a string containing the name of deployment
/// * `registry` - a string containing the url of docker registry
/// * `repository_id` - An Option<&str> containing either a repository id of
///   type string or `None`
/// * `image_name` - An Option<&str> containing either an image name of type
///   string or `None`
/// * `image_tag` - a string containing tags of docker image
///
/// # Returns
/// This function returns Result<Uuid, Error> containing an uuid of the
/// deployment or an error
///
/// [`Transaction`]: Transaction
pub async fn create_deployment_in_organisation(
	connection: &mut <Database as sqlx::Database>::Connection,
	organisation_id: &[u8],
	name: &str,
	registry: &str,
	repository_id: Option<&str>,
	image_name: Option<&str>,
	image_tag: &str,
	region: &str,
	domain_name: Option<&str>,
	horizontal_scale: u64,
	machine_type: &DeploymentMachineType,
	config: &Settings,
) -> Result<Uuid, Error> {
	// As of now, only our custom registry is allowed
	// Docker hub will also be allowed in the near future
	match registry {
		"registry.patr.cloud" => (),
		_ => {
			Error::as_result()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())?;
		}
	}

	if let Some(domain_name) = domain_name {
		if !validator::is_deployment_entry_point_valid(domain_name) {
			return Err(Error::empty()
				.status(400)
				.body(error!(INVALID_DOMAIN_NAME).to_string()));
		}
	}

	let deployment_uuid = db::generate_new_resource_id(connection).await?;
	let deployment_id = deployment_uuid.as_bytes();

	db::create_resource(
		connection,
		deployment_id,
		&format!("Deployment: {}", name),
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(rbac::resource_types::DEPLOYMENT)
			.unwrap(),
		organisation_id,
		get_current_time_millis(),
	)
	.await?;

	if registry == "registry.patr.cloud" {
		if let Some(repository_id) = repository_id {
			let repository_id = hex::decode(repository_id)
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())?;

			db::create_deployment_with_internal_registry(
				connection,
				deployment_id,
				name,
				&repository_id,
				image_tag,
				region,
				domain_name,
				horizontal_scale,
				machine_type,
			)
			.await?;
		} else {
			return Err(Error::empty()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string()));
		}
	} else if let Some(image_name) = image_name {
		db::create_deployment_with_external_registry(
			connection,
			deployment_id,
			name,
			registry,
			image_name,
			image_tag,
			region,
			domain_name,
			horizontal_scale,
			machine_type,
		)
		.await?;
	} else {
		return Err(Error::empty()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string()));
	}

	// Deploy the app as soon as it's created, so that any existing images can
	// be deployed
	service::start_deployment(connection, deployment_id, config).await?;

	Ok(deployment_uuid)
}

pub async fn start_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
	config: &Settings,
) -> Result<(), Error> {
	let deployment = db::get_deployment_by_id(connection, deployment_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let (provider, region) = deployment
		.region
		.split_once('-')
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	let image_id = if let Some(deployed_image) = deployment.deployed_image {
		deployed_image
	} else {
		deployment.get_full_image(connection).await?
	};
	let config = config.clone();
	let region = region.to_string();
	let deployment_id = deployment.id;

	db::update_deployment_deployed_image(
		connection,
		&deployment_id,
		Some(&image_id),
	)
	.await?;

	match provider.parse() {
		Ok(CloudPlatform::DigitalOcean) => {
			task::spawn(async move {
				let result = digitalocean::deploy_container(
					image_id,
					region,
					deployment_id.clone(),
					config,
				)
				.await;

				if let Err(error) = result {
					let _ = update_deployment_status(
						&deployment_id,
						&DeploymentStatus::Errored,
					)
					.await;
					log::info!(
						"Error with the deployment, {}",
						error.get_error()
					);
				}
			});
		}
		Ok(CloudPlatform::Aws) => {
			task::spawn(async move {
				let result = aws::deploy_container(
					image_id,
					region,
					deployment_id.clone(),
					config,
				)
				.await;

				if let Err(error) = result {
					let _ = update_deployment_status(
						&deployment_id,
						&DeploymentStatus::Errored,
					)
					.await;
					log::info!(
						"Error with the deployment, {}",
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

pub async fn stop_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
	config: &Settings,
) -> Result<(), Error> {
	log::trace!("Getting deployment id from db");
	let deployment = db::get_deployment_by_id(connection, deployment_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let (provider, region) = deployment
		.region
		.split_once('-')
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;
	log::trace!("removing the deployed image info from db");
	db::update_deployment_deployed_image(connection, deployment_id, None)
		.await?;

	match provider.parse() {
		Ok(CloudPlatform::DigitalOcean) => {
			log::trace!("deleting the deployment from digitalocean");
			digitalocean::delete_deployment(connection, deployment_id, config)
				.await?;
		}
		Ok(CloudPlatform::Aws) => {
			log::trace!("deleting the deployment from aws");
			aws::delete_deployment(connection, deployment_id, region, config)
				.await?;
		}
		_ => {
			return Err(Error::empty()
				.status(500)
				.body(error!(SERVER_ERROR).to_string()));
		}
	}
	db::update_deployment_status(
		connection,
		deployment_id,
		&DeploymentStatus::Stopped,
	)
	.await?;

	Ok(())
}

pub async fn get_deployment_container_logs(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
	config: &Settings,
) -> Result<String, Error> {
	let deployment = db::get_deployment_by_id(connection, deployment_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;
	log::trace!("get the deployment id from db");

	let (provider, _) = deployment
		.region
		.split_once('-')
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	let logs = match provider.parse() {
		Ok(CloudPlatform::DigitalOcean) => {
			log::trace!("getting logs from digitalocean deployment");
			digitalocean::get_container_logs(connection, deployment_id, config)
				.await?
		}
		Ok(CloudPlatform::Aws) => {
			log::trace!("getting logs from aws deployment");
			aws::get_container_logs(connection, deployment_id, config).await?
		}
		_ => {
			return Err(Error::empty()
				.status(500)
				.body(error!(SERVER_ERROR).to_string()));
		}
	};

	Ok(logs)
}

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
		&organisation_id,
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
		&database_plan,
		&format!("{}-{}", provider, region),
		"",
		0,
		"",
		"",
		&organisation_id,
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

	db::update_managed_database_status(
		connection,
		database_id,
		&ManagedDatabaseStatus::Deleted,
	)
	.await?;
	Ok(())
}

pub async fn set_environment_variables_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
	environment_variables: &[(String, String)],
) -> Result<(), Error> {
	db::remove_all_environment_variables_for_deployment(
		connection,
		deployment_id,
	)
	.await?;

	for (key, value) in environment_variables {
		db::add_environment_variable_for_deployment(
			connection,
			deployment_id,
			key,
			value,
		)
		.await?;
	}

	Ok(())
}

async fn add_cname_record(
	sub_domain: &str,
	target: &str,
	config: &Settings,
	proxied: bool,
) -> Result<(), Error> {
	let full_domain = if sub_domain.ends_with(".patr.cloud") {
		sub_domain.to_string()
	} else {
		format!("{}.patr.cloud", sub_domain)
	};
	let credentials = Credentials::UserAuthToken {
		token: config.cloudflare.api_token.clone(),
	};
	let client = if let Ok(client) = CloudflareClient::new(
		credentials,
		HttpApiClientConfig::default(),
		Environment::Production,
	) {
		client
	} else {
		return Err(Error::empty());
	};
	let zone_identifier = client
		.request(&ListZones {
			params: ListZonesParams {
				name: Some(String::from("patr.cloud")),
				..Default::default()
			},
		})
		.await?
		.result
		.into_iter()
		.next()
		.status(500)?
		.id;
	let zone_identifier = zone_identifier.as_str();
	let expected_dns_record = DnsContent::CNAME {
		content: String::from(target),
	};
	let response = client
		.request(&ListDnsRecords {
			zone_identifier,
			params: ListDnsRecordsParams {
				name: Some(full_domain.clone()),
				..Default::default()
			},
		})
		.await?;
	let dns_record = response.result.into_iter().find(|record| {
		if let DnsContent::CNAME { .. } = record.content {
			record.name == full_domain
		} else {
			false
		}
	});
	if let Some(record) = dns_record {
		if let DnsContent::CNAME { content } = record.content {
			if content != target {
				client
					.request(&UpdateDnsRecord {
						zone_identifier,
						identifier: record.id.as_str(),
						params: UpdateDnsRecordParams {
							content: expected_dns_record,
							name: &full_domain,
							proxied: Some(proxied),
							ttl: Some(1),
						},
					})
					.await?;
			}
		}
	} else {
		// Create
		client
			.request(&CreateDnsRecord {
				zone_identifier,
				params: CreateDnsRecordParams {
					content: expected_dns_record,
					name: sub_domain,
					ttl: Some(1),
					priority: None,
					proxied: Some(proxied),
				},
			})
			.await?;
	}
	Ok(())
}

async fn delete_docker_image(
	deployment_id_string: &str,
	image_id: &str,
) -> Result<(), Error> {
	let docker = Docker::new();

	docker
		.images()
		.get(format!(
			"registry.digitalocean.com/patr-cloud/{}:latest",
			deployment_id_string
		))
		.delete()
		.await?;

	docker.images().get(image_id).delete().await?;

	Ok(())
}

async fn update_deployment_status(
	deployment_id: &[u8],
	status: &DeploymentStatus,
) -> Result<(), sqlx::Error> {
	let app = service::get_app();

	db::update_deployment_status(
		app.database.acquire().await?.deref_mut(),
		deployment_id,
		status,
	)
	.await?;

	Ok(())
}

async fn tag_docker_image(
	image_id: &str,
	new_repo_name: &str,
) -> Result<(), Error> {
	let docker = Docker::new();
	docker
		.images()
		.get(image_id)
		.tag(
			&TagOptions::builder()
				.repo(new_repo_name)
				.tag("latest")
				.build(),
		)
		.await?;

	Ok(())
}

async fn pull_image_from_registry(
	image_id: &str,
	config: &Settings,
) -> Result<(), Error> {
	let app = service::get_app().clone();
	let god_username = db::get_user_by_user_id(
		app.database.acquire().await?.deref_mut(),
		rbac::GOD_USER_ID.get().unwrap().as_bytes(),
	)
	.await?
	.status(500)?
	.username;

	// generate token as password
	let iat = get_current_time().as_secs();
	let token = RegistryToken::new(
		config.docker_registry.issuer.clone(),
		iat,
		god_username.clone(),
		config,
		vec![RegistryTokenAccess {
			r#type: "repository".to_string(),
			name: image_id.to_string(),
			actions: vec!["pull".to_string()],
		}],
	)
	.to_string(
		config.docker_registry.private_key.as_ref(),
		config.docker_registry.public_key_der(),
	)?;

	// get token object using the above token string
	let registry_auth = RegistryAuth::builder()
		.username(god_username)
		.password(token)
		.build();

	let docker = Docker::new();
	let mut stream = docker.images().pull(
		&PullOptions::builder()
			.image(image_id)
			.auth(registry_auth)
			.build(),
	);

	while stream.next().await.is_some() {}

	Ok(())
}

async fn update_managed_database_status(
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

async fn update_managed_database_credentials_for_database(
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
pub async fn get_dns_records_for_deployments(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
) -> Result<Vec<CNameRecord>, Error> {
	let deployment = db::get_deployment_by_id(connection, deployment_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let (provider, region) = deployment
		.region
		.split_once('-')
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	let domain_name = deployment
		.domain_name
		.status(400)
		.body(error!(INVALID_DOMAIN_NAME).to_string())?;

	match provider.parse() {
		Ok(CloudPlatform::DigitalOcean) => {
			let app_id = deployment
				.digital_ocean_app_id
				.status(404)
				.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;
			log::trace!("getting domain for digitalocean deployment");
			let cname_records = digitalocean::get_dns_records_for_deployments(
				&domain_name,
				&app_id,
			)
			.await?;

			Ok(cname_records)
		}
		Ok(CloudPlatform::Aws) => {
			log::trace!("getting domain for aws deployment");
			let cname_records = aws::get_dns_records_for_deployments(
				deployment_id,
				region,
				&domain_name,
			)
			.await?;

			Ok(cname_records)
		}
		_ => Err(Error::empty()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())),
	}
}

pub async fn get_domain_validation_status(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
) -> Result<bool, Error> {
	let deployment = db::get_deployment_by_id(connection, deployment_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let (provider, region) = deployment
		.region
		.split_once('-')
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	let domain_name = deployment
		.domain_name
		.status(400)
		.body(error!(INVALID_DOMAIN_NAME).to_string())?;

	match provider.parse() {
		Ok(CloudPlatform::DigitalOcean) => {
			Ok(reqwest::get(format!("https://{}", domain_name))
				.await?
				.status()
				.is_success())
		}
		Ok(CloudPlatform::Aws) => {
			log::trace!("checking domain validation for aws deployment");
			aws::is_custom_domain_validated(deployment_id, region, &domain_name)
				.await
		}
		_ => Err(Error::empty()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())),
	}
}
