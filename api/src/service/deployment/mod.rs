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
#[allow(clippy::wildcard_in_or_patterns)]
pub async fn create_deployment_in_organisation(
	connection: &mut <Database as sqlx::Database>::Connection,
	organisation_id: &[u8],
	name: &str,
	registry: &str,
	repository_id: Option<&str>,
	image_name: Option<&str>,
	image_tag: &str,
) -> Result<Uuid, Error> {
	// As of now, only our custom registry is allowed
	// Docker hub will also be allowed in the near future
	match registry {
		"registry.patr.cloud" => (),
		"registry.hub.docker.com" | _ => {
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

/*
Documentation for functions yet to come:


fn update_configuration_for_deployment:
/// # Description
/// This function updates the deployment configuration
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `deployment_id` -  an unsigned 8 bit integer array containing the id of
///   deployment
/// * `exposed_ports` - an unsigned 16 bit integer array containing the exposed
///   ports of deployment
/// * `environment_variables` - a string containing the url of docker registry
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


fn create_deployment_upgrade_path_in_organisation:
/// # Description
/// This function creates the deployment according to the upgrade-path
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `organisation_id` -  an unsigned 8 bit integer array containing the id of
///   organisation
/// * `name` - a string containing the name of deployment
/// * `machine_types` - an array of type [`MachineType`] containing the details
///   about machine type
/// * `default_machine_type` - a default configuration of type ['MachineType`]
///
/// # Returns
/// This function returns Result<Uuid, Error> containing an uuid of the
/// deployment or an error
///
/// [`Transaction`]: Transaction
/// [`MachineType`]: MachineType


fn update_deployment_upgrade_path:
/// # Description
/// This function updates the deployment according to the upgrade-path
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `upgrade_path_id` -  an unsigned 8 bit integer array containing the id of
///   the upgrade path
/// * `name` - a string containing name of the deployment
/// * `machine_types` - an array of type [`MachineType`] containing the details
///   about machine type
///
/// # Returns
/// This function returns `Result<(), Error>` containing an empty response or an
/// error
///
/// [`Transaction`]: Transaction
/// [`MachineType`]: MachineType


fn create_deployment_entry_point_in_organisation:
/// # Description
/// This function creates the deployment entry point for the deployment
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `organisation_id` -  an unsigned 8 bit integer array containing the id of
///   organisation
/// * `sub_domain` - a string containing the sub domain for deployment
/// * `domain_id` - An unsigned 8 bit integer array containing id of
///   organisation domain
/// * `path` - a string containing the path for the deployment
/// * `entry_point_type` - a string containing the type of entry point
/// * `deployment_id` - an Option<&str> containing an unsigned 8 bit integer
///   array containing
/// the id of deployment or `None`
/// * `deployment_port` - an Option<u16> containing an unsigned 16 bit integer
///   containing port
/// of deployment or an `None`
/// * `url` - an Option<&str> containing a string of the url for the image to be
///   deployed
///
/// # Returns
/// This function returns `Result<uuid, Error>` containing uuid of the entry
/// point or an error
///
/// [`Transaction`]: Transaction
/// [`MachineType`]: MachineType


fn update_deployment_entry_point:
/// # Description
/// This function updates the deployment entry point for the deployment
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `entry_point_id` - an unsigned
/// * `entry_point_type` - a string containing the type of entry point
/// * `deployment_id` - an Option<&str> containing an unsigned 8 bit integer
///   array containing
/// the id of deployment or `None`
/// * `deployment_port` - an Option<u16> containing an unsigned 16 bit integer
///   containing port
/// of deployment or an `None`
/// * `url` - an Option<&str> containing a string of the url for the image to be
///   deployed
///
/// # Returns
/// This function returns `Result<uuid, Error>` containing uuid of the entry
/// point or an error
///
/// [`Transaction`]: Transaction
/// [`MachineType`]: MachineType
*/
