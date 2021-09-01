use std::{process::Stdio, time::Duration};

use eve_rs::AsError;
use lightsail::model::{
	CertificateStatus,
	Container,
	ContainerServiceDeploymentRequest,
	ContainerServiceDeploymentState,
	ContainerServicePowerName,
	ContainerServiceProtocol,
	ContainerServiceState,
	EndpointRequest,
};
use tokio::{process::Command, time};

use crate::{
	db,
	error,
	models::db_mapping::{CloudPlatform, DeploymentStatus},
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn deploy_container(
	image_id: String,
	region: String,
	deployment_id: Vec<u8>,
	config: Settings,
) -> Result<(), Error> {
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
			&client,
		)
		.await?
	};

	log::trace!("Creating container deployment");
	deploy_application(&deployment_id_string, &client).await?;
	log::trace!("App deployed");

	log::trace!("default url is {}", default_url);
	log::trace!("checking deployment status");
	wait_for_deployment(&deployment_id_string, &client).await?;

	log::trace!("updating DNS");
	super::add_cname_record(&deployment_id_string, &default_url, &config)
		.await?;
	log::trace!("DNS Updated");
	let (cname, value) =
		create_certificate_if_not_available(&deployment_id_string, &client)
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
	)
	.await?;
	log::trace!("cname record updated");
	// update container service with patr domain
	log::trace!("waiting for certificate to be validated");
	wait_for_certificate_validation(&deployment_id_string, &client).await?;
	log::trace!("certificate validated");
	log::trace!("updating container service with patr domain");
	update_container_service_with_patr_domain(&deployment_id_string, &client)
		.await?;
	log::trace!("container service updated with patr domain");

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
	_config: &Settings,
	region: &str,
) -> Result<(), Error> {
	// Get credentails for aws lightsail
	log::trace!("getting credentials from lightsail");
	let client = get_lightsail_client(region);

	// certificate needs to be detached inorder to get deleted but there is no
	// endpoint to detach the certificate

	log::trace!("deleting deployment");

	client
		.delete_container_service()
		.set_service_name(Some(hex::encode(&deployment_id)))
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

pub(super) fn get_lightsail_client(region: &str) -> lightsail::Client {
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
	client: &lightsail::Client,
) -> Result<String, Error> {
	let created_service = client
		.create_container_service()
		.set_service_name(Some(deployment_id.to_string()))
		.scale(1) //setting the default number of containers to 1
		.power(ContainerServicePowerName::Micro) // for now fixing the power of container -> Micro
		.send()
		.await
		.map_err(|err| {
			log::error!("Error during creation of service, {}", err);
			err
		})?;

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
	deployment_id: &str,
	client: &lightsail::Client,
) -> Result<(), Error> {
	let container_image_name =
		get_latest_image_name(deployment_id, client).await?;
	log::trace!("adding image to deployment container");

	let deployment_container = Container::builder()
		// image naming convention -> :service_name.label_name.
		.image(container_image_name)
		.ports("80", ContainerServiceProtocol::Http)
		.build();

	// public endpoint request
	let public_endpoint_request = EndpointRequest::builder()
		.container_name(deployment_id)
		.container_port(80)
		.build();

	// create deployment request
	let deployment_request = ContainerServiceDeploymentRequest::builder()
		.containers(deployment_id, deployment_container)
		.set_public_endpoint(Some(public_endpoint_request))
		.build();
	log::trace!("deployment request created");

	let _ = client
		.create_container_service_deployment()
		.set_containers(deployment_request.containers)
		.set_service_name(Some(deployment_id.to_string()))
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
	deployment_id: &str,
	client: &lightsail::Client,
) -> Result<(String, String), Error> {
	let domain_name = format!("{}.patr.cloud", deployment_id);
	let cert_name = format!("{}-certificate", deployment_id);
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

async fn wait_for_certificate_validation(
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
