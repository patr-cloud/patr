use std::{process::Stdio, time::Duration};

use eve_rs::AsError;
use lightsail::model::{
	Container, ContainerServiceDeploymentRequest,
	ContainerServiceDeploymentState, ContainerServicePowerName,
	ContainerServiceProtocol, ContainerServiceState, EndpointRequest,
};
use tokio::{process::Command, time};

use crate::{
	error,
	models::db_mapping::DeploymentStatus,
	service::{
		delete_docker_image, pull_image_from_registry, tag_docker_image,
		update_deployment_status, update_dns,
	},
	utils::{settings::Settings, Error},
};

pub async fn deploy_container_on_aws(
	image_name: String,
	tag: String,
	deployment_id: Vec<u8>,
	config: Settings,
) -> Result<(), Error> {
	let deployment_id_string = hex::encode(&deployment_id);

	log::trace!("Deploying deployment: {}", deployment_id_string);
	let _ = update_deployment_status(&deployment_id, &DeploymentStatus::Pushed)
		.await;

	log::trace!("Pulling image from registry");
	pull_image_from_registry(&image_name, &tag, &config).await?;
	log::trace!("Image pulled");

	// new name for the docker image
	let new_repo_name = format!("patr-cloud/{}", deployment_id_string);

	log::trace!("Pushing to {}", new_repo_name);

	// rename the docker image with the digital ocean registry url
	tag_docker_image(&image_name, &tag, &new_repo_name).await?;
	log::trace!("Image tagged");

	// Get credentails for aws lightsail
	let client = lightsail::Client::from_env();

	let label_name = "latest".to_string();

	let _ =
		update_deployment_status(&deployment_id, &DeploymentStatus::Deploying)
			.await;

	let app_exists = app_exists(&deployment_id_string, &client).await?;
	let default_url = if let Some(default_url) = app_exists {
		push_image_to_lightsail(
			&deployment_id_string,
			&new_repo_name,
			&label_name,
		)
		.await?;
		log::trace!("pushed the container into aws registry");
		default_url
	} else {
		// create container service
		log::trace!("creating new container service");
		create_container_service(
			&deployment_id_string,
			&label_name,
			&new_repo_name,
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
	update_dns(&deployment_id_string, &default_url, &config).await?;
	log::trace!("DNS Updated");
	time::sleep(Duration::from_secs(5)).await;
	log::trace!("creating certificate");
	create_certificate_for_deployment(&deployment_id_string, &client).await?;
	// update container service with patr domain
	update_container_service_with_patr_domain(&deployment_id_string, &client)
		.await?;

	let _ =
		update_deployment_status(&deployment_id, &DeploymentStatus::Running)
			.await;
	let _ = delete_docker_image(&deployment_id_string, &image_name, &tag).await;
	log::trace!("Docker image deleted");
	Ok(())
}

async fn create_container_service(
	deployment_id: &str,
	label_name: &str,
	new_repo_name: &str,
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

	push_image_to_lightsail(deployment_id, new_repo_name, label_name).await?;
	log::trace!("pushed the container into aws registry");

	let default_url = created_service
		.container_service
		.map(|service| service.url)
		.flatten()
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;
	Ok(default_url)
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
) -> Result<(), Error> {
	let output = Command::new("aws")
		.arg("lightsail")
		.arg("push-container-image")
		.arg("--service-name")
		.arg(deployment_id)
		.arg("--image")
		.arg(format!("{}:latest", new_repo_name))
		.arg("--region")
		.arg("us-east-2")
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
		.public_domain_names(&sub_domain, vec![sub_domain.clone()])
		.send()
		.await?;

	Ok(())
}

async fn create_certificate_for_deployment(
	deployment_id: &str,
	client: &lightsail::Client,
) -> Result<(), Error> {
	client
		.create_certificate()
		.certificate_name(format!("{}.patr.cloud", deployment_id))
		.domain_name(format!("{}.patr.cloud", deployment_id))
		.send()
		.await?;
	log::trace!("certificate created");
	Ok(())
}
