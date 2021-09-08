use std::{ops::DerefMut, process::Stdio, time::Duration};

use eve_rs::AsError;
use lightsail::{
	model::{
		CertificateStatus,
		Container,
		ContainerServiceDeploymentRequest,
		ContainerServiceDeploymentState,
		ContainerServicePowerName,
		ContainerServiceProtocol,
		ContainerServiceState,
		EndpointRequest,
	},
	SdkError,
};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use tokio::{process::Command, task, time};

use crate::{
	db,
	error,
	models::db_mapping::{
		CNameRecord,
		CloudPlatform,
		DeploymentMachineType,
		DeploymentStatus,
		ManagedDatabaseEngine,
		ManagedDatabasePlan,
		ManagedDatabaseStatus,
	},
	service,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn deploy_container(
	image_id: String,
	region: String,
	deployment_id: Vec<u8>,
	config: Settings,
) -> Result<(), Error> {
	let deployment = db::get_deployment_by_id(
		service::get_app().database.acquire().await?.deref_mut(),
		&deployment_id,
	)
	.await?
	.status(500)
	.body(error!(SERVER_ERROR).to_string())?;

	let deployment_id_string = hex::encode(&deployment_id);
	log::trace!("Deploying deployment: {}", deployment_id_string);
	let _ = super::update_deployment_status(
		&deployment_id,
		&DeploymentStatus::Pushed,
	)
	.await;

	log::trace!("Pulling image from registry");
	super::pull_image_from_registry(&image_id, &config).await?;
	log::trace!("Image pulled");

	// new name for the docker image
	let new_repo_name = format!("patr-cloud/{}", deployment_id_string);

	log::trace!("Pushing to {}", new_repo_name);

	// rename the docker image with the digital ocean registry url
	super::tag_docker_image(&image_id, &new_repo_name).await?;
	log::trace!("Image tagged");

	// Get credentails for aws lightsail
	let client = get_lightsail_client(&region);

	let label_name = "latest".to_string();

	let _ = super::update_deployment_status(
		&deployment_id,
		&DeploymentStatus::Deploying,
	)
	.await;

	let app_exists = app_exists(&deployment_id_string, &client).await?;
	let default_url = if let Some(default_url) = app_exists {
		push_image_to_lightsail(
			&deployment_id_string,
			&new_repo_name,
			&label_name,
			&region,
		)
		.await?;
		log::trace!("pushed the container into aws registry");
		default_url.replace("https://", "").replace("/", "")
	} else {
		// create container service
		log::trace!("creating new container service");
		create_container_service(
			&deployment_id_string,
			&label_name,
			&new_repo_name,
			&region,
			deployment.horizontal_scale,
			&deployment.machine_type,
			&client,
		)
		.await?
	};

	log::trace!("Creating container deployment");
	deploy_application(&deployment_id, &deployment_id_string, &client).await?;
	log::trace!("App deployed");

	log::trace!("default url is {}", default_url);
	log::trace!("checking deployment status");
	wait_for_deployment(&deployment_id_string, &client).await?;
	log::trace!("updating DNS");
	super::add_cname_record(&deployment_id_string, &default_url, &config, true)
		.await?;
	log::trace!("DNS Updated");
	let domain_name = format!("{}.patr.cloud", deployment_id_string);
	let (cname, value) = create_certificate_if_not_available(
		&deployment_id_string,
		&domain_name,
		&client,
	)
	.await?;
	log::trace!("updating cname record");

	super::add_cname_record(
		if cname.ends_with('.') {
			&cname[0..cname.len() - 1]
		} else {
			&cname
		},
		if value.ends_with('.') {
			&value[0..value.len() - 1]
		} else {
			&value
		},
		&config,
		false,
	)
	.await?;
	log::trace!("cname record updated");
	// update container service with patr domain
	log::trace!("waiting for certificate to be validated");
	wait_for_patr_certificate_validation(&deployment_id_string, &client)
		.await?;
	log::trace!("certificate validated");
	log::trace!("updating container service with patr domain");
	update_container_service_with_patr_domain(&deployment_id_string, &client)
		.await?;
	log::trace!("container service updated with patr domain");
	let custom_domain = db::get_deployment_by_id(
		service::get_app().database.acquire().await?.deref_mut(),
		&deployment_id,
	)
	.await?
	.map(|deployment| deployment.domain_name)
	.flatten();

	if let Some(domain) = custom_domain {
		log::trace!(
			"custom domain present, updating patr service with custom domain"
		);
		let cert_name = format!("{}-custom", deployment_id_string);
		create_certificate_if_not_available(&cert_name, &domain, &client)
			.await?;
		log::trace!("container service updated with custom domain");
	};

	let _ = super::update_deployment_status(
		&deployment_id,
		&DeploymentStatus::Running,
	)
	.await;
	let _ = super::delete_docker_image(&deployment_id_string, &image_id).await;
	log::trace!("Docker image deleted");
	Ok(())
}

pub(super) async fn delete_deployment(
	_connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
	region: &str,
	_config: &Settings,
) -> Result<(), Error> {
	// Get credentails for aws lightsail
	log::trace!("getting credentials from lightsail");
	let client = get_lightsail_client(region);
	let deployment_id_string = hex::encode(deployment_id);

	// certificate needs to be detached inorder to get deleted but there is no
	// endpoint to detach the certificate
	log::trace!("deleting deployment");

	client
		.delete_container_service()
		.service_name(&deployment_id_string)
		.send()
		.await
		.map_err(|err| {
			log::error!("Error during deletion of service, {}", err);
			err
		})?;
	log::trace!("deployment deleted successfully!");

	Ok(())
}

pub(super) async fn get_container_logs(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
	_config: &Settings,
) -> Result<String, Error> {
	log::info!("retreiving deployment info from db");
	let deployment = db::get_deployment_by_id(connection, deployment_id)
		.await?
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	let (provider, region) = deployment
		.region
		.split_once('-')
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	if provider.parse().ok() != Some(CloudPlatform::Aws) {
		log::error!(
			"Provider in deployment region is {}, but AWS logs were requested.",
			provider
		);
		return Err(Error::empty()
			.status(500)
			.body(error!(SERVER_ERROR).to_string()));
	}

	// Get credentails for aws lightsail
	log::trace!("getting credentails from aws lightsail");
	let client = get_lightsail_client(region);
	log::info!("getting logs from aws");
	let logs = client
		.get_container_log()
		.set_service_name(Some(hex::encode(&deployment_id)))
		.set_container_name(Some(hex::encode(&deployment_id)))
		.send()
		.await
		.map_err(|err| {
			log::error!("Error during deletion of service, {}", err);
			err
		})?
		.log_events
		.map(|events| {
			events
				.into_iter()
				.filter_map(|event| event.message)
				.collect::<Vec<_>>()
				.join("\n")
		})
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;
	log::info!("logs retreived successfully!");
	Ok(logs)
}

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
	let client = get_lightsail_client(region);

	let username = format!("user_{}", db_name);
	let password = thread_rng()
		.sample_iter(&Alphanumeric)
		.take(8)
		.map(char::from)
		.collect::<String>();

	log::trace!("sending the create db cluster request to aws");
	client
		.create_relational_database()
		.master_database_name(db_name)
		.master_username(&username)
		.master_user_password(&password)
		.publicly_accessible(true)
		.relational_database_blueprint_id(format!("{}_{}", engine, version))
		.relational_database_bundle_id(database_plan.as_aws_plan()?)
		.relational_database_name(hex::encode(database_id))
		.send()
		.await?;
	log::trace!("database created");

	let database_id = database_id.to_vec();
	let region = region.to_string();

	task::spawn(async move {
		let result = update_database_cluster_credentials(
			database_id.clone(),
			region,
			username,
			password,
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
	database_id: &[u8],
	region: &str,
) -> Result<(), Error> {
	let client = get_lightsail_client(region);

	let database_cluster = client
		.get_relational_database()
		.relational_database_name(hex::encode(database_id))
		.send()
		.await;

	if database_cluster.is_err() {
		return Ok(());
	}

	client
		.delete_relational_database()
		.relational_database_name(hex::encode(database_id))
		.send()
		.await?;

	Ok(())
}

pub(super) async fn is_custom_domain_validated(
	deployment_id: &[u8],
	region: &str,
	domain_name: &str,
) -> Result<bool, Error> {
	let client = get_lightsail_client(region);
	let deployment_id_string = hex::encode(deployment_id);

	let certificate_status = client
		.get_certificates()
		.certificate_name(format!(
			"{}-custom-certificate",
			deployment_id_string
		))
		.include_certificate_details(true)
		.send()
		.await?
		.certificates
		.map(|certificate_summary_list| {
			certificate_summary_list.into_iter().next()
		})
		.flatten()
		.map(|certificate_summary| certificate_summary.certificate_detail)
		.flatten()
		.map(|cert| cert.status)
		.flatten()
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	match certificate_status {
		CertificateStatus::Issued => {
			let custom_domain_exists = client
				.get_container_services()
				.service_name(&deployment_id_string)
				.send()
				.await?
				.container_services
				.map(|services| services.into_iter().next())
				.flatten()
				.map(|service| service.public_domain_names.unwrap_or_default())
				.status(500)
				.body(error!(SERVER_ERROR).to_string())?
				.contains_key(&format!(
					"{}-custom-certificate",
					deployment_id_string
				));
			if !custom_domain_exists {
				client
					.update_container_service()
					.service_name(&deployment_id_string)
					.is_disabled(false)
					.public_domain_names(
						format!("{}-custom-certificate", deployment_id_string),
						vec![domain_name.to_string()],
					)
					.send()
					.await?;
			}
			Ok(true)
		}
		CertificateStatus::PendingValidation => Ok(false),
		_ => Err(Error::empty()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())),
	}
}

pub(super) async fn get_dns_records_for_deployments(
	deployment_id: &[u8],
	region: &str,
	domain_name: &str,
) -> Result<Vec<CNameRecord>, Error> {
	let client = get_lightsail_client(region);

	log::trace!("getting deployment url from lightsail");
	let deployment_url = app_exists(&hex::encode(deployment_id), &client)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let cert_name =
		format!("{}-custom-certificate", hex::encode(deployment_id));
	log::trace!("retreive certificate for deployment");
	let (cname, value) =
		get_cname_and_value_from_aws(&cert_name, &client).await?;

	let cname = if cname.ends_with('.') {
		&cname[0..cname.len() - 1]
	} else {
		&cname
	};
	let value = if value.ends_with('.') {
		&value[0..value.len() - 1]
	} else {
		&value
	};
	let cname_records = vec![
		CNameRecord {
			cname: domain_name.to_string(),
			value: deployment_url,
		},
		CNameRecord {
			cname: cname.to_string(),
			value: value.to_string(),
		},
	];

	Ok(cname_records)
}

async fn update_database_cluster_credentials(
	database_id: Vec<u8>,
	region: String,
	username: String,
	password: String,
) -> Result<(), Error> {
	let client = get_lightsail_client(&region);

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

	super::update_managed_database_credentials_for_database(
		&database_id,
		&host,
		port,
		&username,
		&password,
	)
	.await?;

	log::trace!("updating to the db status to running");
	// wait for database to start
	super::update_managed_database_status(
		&database_id,
		&ManagedDatabaseStatus::Running,
	)
	.await?;
	log::trace!("database successfully updated");

	Ok(())
}

fn get_lightsail_client(region: &str) -> lightsail::Client {
	let deployment_region = lightsail::Region::new(region.to_string());
	let client_builder = lightsail::Config::builder()
		.region(Some(deployment_region))
		.build();
	lightsail::Client::from_conf(client_builder)
}

async fn create_container_service(
	deployment_id: &str,
	label_name: &str,
	new_repo_name: &str,
	region: &str,
	horizontal_scale: i16,
	machine_type: &DeploymentMachineType,
	client: &lightsail::Client,
) -> Result<String, Error> {
	log::trace!("checking if the service exists or is in the process of getting deleted");
	let created_service = loop {
		let created_result = client
			.create_container_service()
			.set_service_name(Some(deployment_id.to_string()))
			.scale(horizontal_scale.into())
			.power(match machine_type {
				DeploymentMachineType::Micro => ContainerServicePowerName::Nano,
				DeploymentMachineType::Small => {
					ContainerServicePowerName::Small
				}
				DeploymentMachineType::Medium => {
					ContainerServicePowerName::Medium
				}
				DeploymentMachineType::Large => {
					ContainerServicePowerName::Large
				}
			})
			.send()
			.await;
		match created_result {
			Ok(created_service) => break created_service,
			Err(SdkError::ServiceError { err, raw }) => {
				if err.message() == Some(&format!("Please try again when service \"{}\" is done DELETING.", deployment_id)) {
					// If the service is deleting, wait and try again
					time::sleep(Duration::from_millis(1000)).await;
				} else {
					// If there's some other error, return the error
					return Err(SdkError::ServiceError { err, raw }.into());
				}
			}
			Err(error) => {
				return Err(error.into());
			}
		}
	};

	let custom_certificate = client
		.get_certificates()
		.certificate_name(format!("{}-custom-certificate", deployment_id))
		.send()
		.await?
		.certificates
		.map(|services| services.into_iter().next())
		.flatten();
	if custom_certificate.is_some() {
		client
			.delete_certificate()
			.certificate_name(format!("{}-custom-certificate", deployment_id))
			.send()
			.await?;
	}

	loop {
		let container_state = client
			.get_container_services()
			.service_name(deployment_id.to_string())
			.send()
			.await
			.map_err(|err| {
				log::error!(
					"Error during fetching status of deployment, {}",
					err
				);
				err
			})?
			.container_services
			.map(|services| services.into_iter().next())
			.flatten()
			.map(|service| service.state)
			.flatten();

		if let Some(ContainerServiceState::Ready) = container_state {
			break;
		}
		time::sleep(Duration::from_millis(1000)).await;
	}
	log::trace!("container service created");

	push_image_to_lightsail(deployment_id, new_repo_name, label_name, region)
		.await?;
	log::trace!("pushed the container into aws registry");

	let default_url = created_service
		.container_service
		.map(|service| service.url)
		.flatten()
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;
	Ok(default_url.replace("https://", "").replace("/", ""))
}

async fn app_exists(
	deployment_id: &str,
	client: &lightsail::Client,
) -> Result<Option<String>, Error> {
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
		.flatten();

	Ok(default_url)
}

async fn deploy_application(
	deployment_id: &[u8],
	deployment_id_string: &str,
	client: &lightsail::Client,
) -> Result<(), Error> {
	let container_image_name =
		get_latest_image_name(deployment_id_string, client).await?;
	log::trace!("adding image to deployment container");

	let envs = db::get_environment_variables_for_deployment(
		service::get_app().database.acquire().await?.deref_mut(),
		deployment_id,
	)
	.await?
	.into_iter()
	.map(|(key, value)| (key, value))
	.collect();

	let deployment_container = Container::builder()
		// image naming convention -> :service_name.label_name.
		.image(container_image_name)
		.ports("80", ContainerServiceProtocol::Http)
		.set_environment(Some(envs))
		.build();

	// public endpoint request
	let public_endpoint_request = EndpointRequest::builder()
		.container_name(deployment_id_string)
		.container_port(80)
		.build();

	// create deployment request
	let deployment_request = ContainerServiceDeploymentRequest::builder()
		.containers(deployment_id_string, deployment_container)
		.set_public_endpoint(Some(public_endpoint_request))
		.build();
	log::trace!("deployment request created");

	let _ = client
		.create_container_service_deployment()
		.set_containers(deployment_request.containers)
		.set_service_name(Some(deployment_id_string.to_string()))
		.set_public_endpoint(deployment_request.public_endpoint)
		.send()
		.await?;
	log::trace!("created the deployment successfully");

	Ok(())
}

async fn get_latest_image_name(
	deployment_id: &str,
	client: &lightsail::Client,
) -> Result<String, Error> {
	log::trace!("getting container list");
	let container_image = client
		.get_container_images()
		.service_name(deployment_id)
		.send()
		.await?
		.container_images
		.map(|value| value.into_iter().next())
		.flatten()
		.map(|image| image.image)
		.flatten()
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;
	log::trace!("recieved the container images");

	Ok(container_image)
}

async fn push_image_to_lightsail(
	deployment_id: &str,
	new_repo_name: &str,
	label_name: &str,
	region: &str,
) -> Result<(), Error> {
	let output = Command::new("aws")
		.arg("lightsail")
		.arg("push-container-image")
		.arg("--service-name")
		.arg(deployment_id)
		.arg("--image")
		.arg(format!("{}:latest", new_repo_name))
		.arg("--region")
		.arg(region)
		.arg("--label")
		.arg(&label_name)
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()?
		.wait()
		.await?;

	if !output.success() {
		return Err(Error::empty()
			.status(500)
			.body(error!(SERVER_ERROR).to_string()));
	}

	Ok(())
}

async fn wait_for_deployment(
	deployment_id: &str,
	client: &lightsail::Client,
) -> Result<(), Error> {
	loop {
		let deployment_state = client
			.get_container_service_deployments()
			.service_name(deployment_id)
			.send()
			.await?
			.deployments
			.map(|deployments| deployments.into_iter().next())
			.flatten()
			.map(|deployment| deployment.state)
			.flatten();

		if let Some(ContainerServiceDeploymentState::Active) = deployment_state
		{
			return Ok(());
		}
		time::sleep(Duration::from_millis(1000)).await;
	}
}

async fn update_container_service_with_patr_domain(
	deployment_id: &str,
	client: &lightsail::Client,
) -> Result<(), Error> {
	let sub_domain = format!("{}.patr.cloud", deployment_id);
	client
		.update_container_service()
		.service_name(deployment_id)
		.is_disabled(false)
		.public_domain_names(
			format!("{}-certificate", deployment_id),
			vec![sub_domain],
		)
		.send()
		.await?;

	Ok(())
}

async fn create_certificate_if_not_available(
	cert_name: &str,
	domain_name: &str,
	client: &lightsail::Client,
) -> Result<(String, String), Error> {
	let cert_name = format!("{}-certificate", cert_name);
	log::trace!("checking if the certificate exists for the current domain");
	let certificate_info =
		get_cname_and_value_from_aws(&cert_name, client).await;

	if certificate_info.is_ok() {
		log::trace!("certificate exists");
		return certificate_info;
	}
	log::trace!("creating new certificate");
	let certificte_name = client
		.create_certificate()
		.certificate_name(&cert_name)
		.domain_name(domain_name)
		.send()
		.await?
		.certificate
		.map(|certificate_summary| certificate_summary.certificate_name)
		.flatten()
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	let (cname, value) =
		get_cname_and_value_from_aws(&certificte_name, client).await?;

	log::trace!("certificate created");
	Ok((cname, value))
}

async fn get_cname_and_value_from_aws(
	cert_name: &str,
	client: &lightsail::Client,
) -> Result<(String, String), Error> {
	loop {
		let certificate_domain_validation_record = client
			.get_certificates()
			.certificate_name(cert_name)
			.include_certificate_details(true)
			.send()
			.await?
			.certificates
			.map(|certificate_summary_list| {
				certificate_summary_list.into_iter().next()
			})
			.flatten()
			.map(|certificate_summary| certificate_summary.certificate_detail)
			.flatten()
			.map(|cert| cert.domain_validation_records)
			.flatten()
			.map(|domain_validation_record| {
				domain_validation_record.into_iter().next()
			})
			.flatten()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;

		if certificate_domain_validation_record.domain_name.is_some() &&
			certificate_domain_validation_record
				.resource_record
				.is_some()
		{
			let (cname, value) = certificate_domain_validation_record
				.resource_record
				.map(|record| record.name.zip(record.value))
				.flatten()
				.status(500)
				.body(error!(SERVER_ERROR).to_string())?;

			return Ok((cname, value));
		} else if certificate_domain_validation_record.domain_name.is_none() {
			break;
		}
		time::sleep(Duration::from_millis(1000)).await;
	}

	Error::as_result()
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?
}

async fn wait_for_patr_certificate_validation(
	deployment_id_string: &str,
	client: &lightsail::Client,
) -> Result<(), Error> {
	loop {
		let certificate_status = client
			.get_certificates()
			.certificate_name(format!("{}-certificate", deployment_id_string))
			.include_certificate_details(true)
			.send()
			.await?
			.certificates
			.map(|certificate_summary_list| {
				certificate_summary_list.into_iter().next()
			})
			.flatten()
			.map(|certificate_summary| certificate_summary.certificate_detail)
			.flatten()
			.map(|cert| cert.status)
			.flatten()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;

		if certificate_status == CertificateStatus::Issued {
			return Ok(());
		} else if certificate_status == CertificateStatus::PendingValidation {
			time::sleep(Duration::from_millis(1000)).await;
		} else {
			return Err(Error::empty()
				.status(500)
				.body(error!(SERVER_ERROR).to_string()));
		}
	}
}
