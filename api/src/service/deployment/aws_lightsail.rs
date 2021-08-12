use std::process::{Command, Stdio};

use eve_rs::AsError;
use lightsail::model::{
	Container,
	ContainerServiceDeploymentRequest,
	ContainerServicePowerName,
	ContainerServiceProtocol,
	ContainerServiceState,
	EndpointRequest,
};

use crate::{
	error,
	models::db_mapping::DeploymentStatus,
	service::{
		pull_image_from_registry,
		tag_docker_image,
		update_deployment_status,
	},
	utils::{settings::Settings, Error},
};

pub async fn deploy_container_on_aws_lightsail(
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

	let service_name = hex::encode(&deployment_id);
	// TODO: find a better name for label
	let label_name = "deployment_label".to_string();

	// TODO: add region details
	let output = Command::new("aws")
		.arg("lightsail")
		.arg("push-container-image")
		.arg("--service-name")
		.arg(&service_name)
		.arg("--image")
		.arg(&new_repo_name)
		.arg("--region")
		.arg("us-east-2")
		.arg("--label")
		.arg(&label_name)
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()?
		.wait()?;

	if !output.success() {
		return Err(Error::empty()
			.status(500)
			.body(error!(SERVER_ERROR).to_string()));
	}

	log::trace!("pushed the container into aws registry");

	// TODO: add the logic for the case when app exists
	// let app_exists = app_exists(&deployment_id, &config, &client).await?;
	// log::trace!("App exists as {:?}", app_exists);

	let _ =
		update_deployment_status(&deployment_id, &DeploymentStatus::Deploying)
			.await;

	// create container service
	log::trace!("creating container service");
	create_container_service_and_deploy().await?;

	// TODO: use similar logic for further process
	// let app_id = if let Some(app_id) = app_exists {
	// 	// the function to create a new deployment
	// 	redeploy_application(&app_id, &config, &client).await?;
	// 	log::trace!("App redeployed");
	// 	app_id
	// } else {
	// 	// if the app doesn't exists then create a new app
	// 	let app_id = create_app(&deployment_id, &config, &client).await?;
	// 	log::trace!("App created");
	// 	app_id
	// };

	// // wait for the app to be completed to be deployed
	// let default_ingress = wait_for_deploy(&app_id, &config, &client).await;
	// log::trace!("App ingress is at {}", default_ingress);

	// // update DNS
	// update_dns(&deployment_id_string, &default_ingress, &config).await?;
	// log::trace!("DNS Updated");

	// let _ =
	// 	update_deployment_status(&deployment_id, &DeploymentStatus::Running)
	// 		.await;
	// let _ = delete_docker_image(&deployment_id_string, &image_name,
	// &tag).await; log::trace!("Docker image deleted");

	Ok(())
}

pub async fn create_container_service_and_deploy() -> Result<(), Error> {
	let client = lightsail::Client::from_env();

	// get latest container
	// TODO: extract this part to a function so that it can be used by the
	// update part
	let container_info = client
		.get_container_images()
		.service_name("service_name".to_string())
		.send()
		.await?;

	if container_info.container_images.is_none() {
		Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;
	}

	let image_list = container_info.container_images.unwrap();

	let container_image = image_list.get(0);

	if container_image.is_none() {
		Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;
	}

	let container_image = container_image.unwrap();

	let container_image = container_image.image.as_ref();

	let container_image_name = match container_image {
		Some(image) => image,
		None => Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?,
	};

	let deployment_container = Container::builder()
		// image naming convention -> :service_name.label_name.
		.image(container_image_name)
		.ports("80".to_string(), ContainerServiceProtocol::Https)
		.build();

	// public endpoint request
	let public_endpoint_request = EndpointRequest::builder()
		.container_name(container_image_name)
		.container_port(80)
		.build();

	// create deployment request
	let deployment_request = ContainerServiceDeploymentRequest::builder()
		.containers("container_name".to_string(), deployment_container)
		.set_public_endpoint(Some(public_endpoint_request))
		.build();

	let container_service = client
		.create_container_service()
		.set_service_name(Some("set_service_name".to_string()))
		.deployment(deployment_request)
		.scale(1) //setting the default number of containers to 1
		.power(ContainerServicePowerName::Micro) // for now fixing the power of container -> Micro
		.send()
		.await?;

	loop {
		let container_status = client
			.get_container_services()
			.service_name("default-service-1".to_string())
			.send()
			.await?;
		if let Some(container_services) = container_status.container_services {
			if let Some(container_state) = &container_services[0].state {
				if *container_state == ContainerServiceState::Ready {
					break;
				}
			}
		}
	}
	log::trace!("container service created");

	Ok(())
}
