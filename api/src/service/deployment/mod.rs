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
pub use digitalocean::*;
use eve_rs::AsError;
use shiplift::Docker;
use tokio::task;
use uuid::Uuid;

use crate::{
	db,
	error,
	models::{
		db_mapping::{Deployment, DeploymentStatus},
		rbac,
	},
	service,
	utils::{get_current_time_millis, settings::Settings, Error},
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
	let full_image_name;

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
			)
			.await?;

			full_image_name = Deployment {
				id: deployment_id.to_vec(),
				name: name.to_string(),
				registry: registry.to_string(),
				repository_id: Some(repository_id),
				image_name: None,
				image_tag: image_tag.to_string(),
				status: DeploymentStatus::Created,
				deployed_image: None,
				digital_ocean_app_id: None,
				region: region.to_string(),
			}
			.get_full_image(connection)
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
		)
		.await?;

		full_image_name = Deployment {
			id: deployment_id.to_vec(),
			name: name.to_string(),
			registry: registry.to_string(),
			repository_id: None,
			image_name: Some(image_name.to_string()),
			image_tag: image_tag.to_string(),
			status: DeploymentStatus::Created,
			deployed_image: None,
			digital_ocean_app_id: None,
			region: region.to_string(),
		}
		.get_full_image(connection)
		.await?;
	} else {
		return Err(Error::empty()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string()));
	}

	let image_tag = image_tag.to_string();
	let deployment_id = deployment_id.to_vec();

	task::spawn(async move {
		let deploy_result = service::deploy_container_on_digitalocean(
			full_image_name.clone(),
			image_tag,
			deployment_id.clone(),
			service::get_settings().clone(),
		)
		.await;

		let app = service::get_app().database.acquire().await;
		if let Ok(mut connection) = app {
			let _ = if deploy_result.is_ok() {
				db::update_deployment_deployed_image(
					&mut connection,
					&deployment_id,
					&full_image_name,
				)
				.await
			} else {
				update_deployment_status(
					&deployment_id,
					&DeploymentStatus::Errored,
				)
				.await
			};
		}
	});

	Ok(deployment_uuid)
}

pub async fn start_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
	config: &Settings,
) -> Result<(), Error> {
	let deployment = db::get_deployment_by_id(connection, &deployment_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let (provider, region) = deployment
		.region
		.split_once('-')
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	let image_name = if let Some(deployed_image) = deployment.deployed_image {
		deployed_image
	} else {
		deployment.get_full_image(connection).await?
	};
	let image_tag = deployment.image_tag;
	let config = config.clone();
	let region = region.to_string();

	db::update_deployment_deployed_image(
		connection,
		deployment_id,
		&image_name,
	)
	.await?;

	match provider {
		"do" => {
			task::spawn(async move {
				let result = service::deploy_container_on_digitalocean(
					image_name,
					image_tag,
					region,
					deployment.id,
					config,
				)
				.await;

				if let Err(error) = result {
					log::info!(
						"Error with the deployment, {}",
						error.get_error()
					);
				}
			});
		}
		"aws" => {
			task::spawn(async move {
				let result = service::deploy_container_on_aws(
					image_name,
					image_tag,
					deployment.id,
					config,
				)
				.await;

				if let Err(error) = result {
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

pub async fn start_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
	config: &Settings,
) -> Result<(), Error> {
	let deployment = db::get_deployment_by_id(connection, &deployment_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let (provider, region) = deployment
		.region
		.split_once('-')
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	let image_name = if let Some(deployed_image) = deployment.deployed_image {
		deployed_image
	} else {
		deployment.get_full_image(connection).await?
	};
	let image_tag = deployment.image_tag;
	let config = config.clone();

	db::update_deployment_deployed_image(
		connection,
		deployment_id,
		&image_name,
	)
	.await?;

	match provider {
		"do" => task::spawn(async move {
			let result = service::deploy_container_on_digitalocean(
				image_name,
				image_tag,
				deployment.id,
				config,
			)
			.await;

			if let Err(error) = result {
				log::info!("Error with the deployment, {}", error.get_error());
			}
		}),
		"aws" => task::spawn(async move {
			let result = service::deploy_container_on_aws(
				image_name,
				image_tag,
				deployment.id,
				config,
			)
			.await;

			if let Err(error) = result {
				log::info!("Error with the deployment, {}", error.get_error());
			}
		}),
		_ => {
			return Err(Error::empty()
				.status(500)
				.body(error!(SERVER_ERROR).to_string()))
		}
	}

	Ok(())
}

async fn add_cname_record(
	sub_domain: &str,
	target: &str,
	config: &Settings,
) -> Result<(), Error> {
	let full_domain = format!("{}.patr.cloud", sub_domain);
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
							proxied: Some(true),
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
					proxied: Some(true),
				},
			})
			.await?;
	}
	Ok(())
}

async fn delete_docker_image(
	deployment_id_string: &str,
	image_name: &str,
	tag: &str,
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

	docker
		.images()
		.get(format!("{}:{}", image_name, tag))
		.delete()
		.await?;

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
