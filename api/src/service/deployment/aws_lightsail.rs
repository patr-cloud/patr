use std::process::Stdio;

use eve_rs::AsError;
use lightsail::model::{
	Container,
	ContainerServiceDeploymentRequest,
	ContainerServiceHealthCheckConfig,
	ContainerServicePowerName,
	ContainerServiceProtocol,
	ContainerServiceState,
	EndpointRequest,
};
use tokio::process::Command;

use crate::{
	error,
	models::db_mapping::DeploymentStatus,
	service::{
		delete_docker_image,
		pull_image_from_registry,
		tag_docker_image,
		update_deployment_status,
		update_dns,
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

	// Get credentails for aws lightsail
	let client = lightsail::Client::from_env();

	let service_name = hex::encode(&deployment_id);
	// TODO: find a better name for label
	let label_name = "deployment-label".to_string();

	let _ =
		update_deployment_status(&deployment_id, &DeploymentStatus::Deploying)
			.await;

	let app_existence = app_exists(&service_name, &client).await?;
	if app_existence {
		// TODO: add region details and extract this part into a function
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
			.wait()
			.await?;

		if !output.success() {
			return Err(Error::empty()
				.status(500)
				.body(error!(SERVER_ERROR).to_string()));
		}
		log::trace!("pushed the container into aws registry");

		log::trace!("container service exists as {}", &service_name);
		deploy_application(&service_name, &client).await?;
		log::trace!("App deployed");
	} else {
		// create container service
		log::trace!("creating new container service");
		create_container_service_and_deploy(
			&service_name,
			&label_name,
			&new_repo_name,
			&client,
		)
		.await?;
	}
	// wait for the app to be completed to be deployed
	let default_url = get_default_url(&service_name, &client).await?;
	log::trace!("default url is {}", default_url);

	// update DNS
	update_dns(&deployment_id_string, &default_url, &config).await?;
	log::trace!("DNS Updated");

	let _ =
		update_deployment_status(&deployment_id, &DeploymentStatus::Running)
			.await;
	let _ = delete_docker_image(&deployment_id_string, &image_name, &tag).await;
	log::trace!("Docker image deleted");

	Ok(())
}

async fn create_container_service_and_deploy(
	service_name: &str,
	label_name: &str,
	new_repo_name: &str,
	client: &lightsail::Client,
) -> Result<(), Error> {
	let container_service = client
		.create_container_service()
		.set_service_name(Some(service_name.to_string()))
		.scale(1) //setting the default number of containers to 1
		.power(ContainerServicePowerName::Micro) // for now fixing the power of container -> Micro
		// .public_domain_names(
		// 	"patr-cloud".to_string(),
		// 	vec![format!("{}.patr.cloud", service_name)], /* getting this
		// 	                                               * error here: The
		// 	                                               * specified certificate
		// 	                                               * does not exist
		// 	                                               * for cert name
		// 	                                               * patr-cloud for
		// 	                                               * service 00 */
		// )
		.send()
		.await;

	if let Err(error) = container_service {
		log::info!("Error during creation of service, {}", error);
		return Ok(());
	}

	loop {
		let container_status = client
			.get_container_services()
			.service_name(service_name.to_string())
			.send()
			.await;

		if let Err(error) = container_status {
			log::info!("Error during fetching status of deployment, {}", error);

			return Ok(());
		} else if let Ok(container_status) = container_status {
			if let Some(container_services) =
				container_status.container_services
			{
				if let Some(container_state) = &container_services[0].state {
					if *container_state == ContainerServiceState::Ready {
						break;
					}
				}
			}
		}
	}
	log::trace!("container service created");

	// TODO: add region details and extract this part into a function
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
		.wait()
		.await?;

	if !output.success() {
		return Err(Error::empty()
			.status(500)
			.body(error!(SERVER_ERROR).to_string()));
	}
	log::trace!("pushed the container into aws registry");

	log::trace!("creating container deployment");

	deploy_application(&service_name, client).await?;

	Ok(())
}

async fn app_exists(
	service_name: &str,
	client: &lightsail::Client,
) -> Result<bool, Error> {
	let container_service = client
		.get_container_services()
		.service_name(service_name.to_string())
		.send()
		.await;

	if let Err(_) = container_service {
		log::trace!("App not found");
	} else {
		return Ok(true);
	}
	Ok(false)
}

async fn deploy_application(
	service_name: &str,
	client: &lightsail::Client,
) -> Result<(), Error> {
	let deployment_request =
		make_deployment_for_latest_image(service_name, client).await?;
	let _application = client
		.create_container_service_deployment()
		.set_containers(deployment_request.containers)
		.set_service_name(Some(service_name.to_string()))
		.set_public_endpoint(deployment_request.public_endpoint)
		.send()
		.await?;
	log::trace!("created the deployment successfully");

	Ok(())
}

async fn make_deployment_for_latest_image(
	service_name: &str,
	client: &lightsail::Client,
) -> Result<ContainerServiceDeploymentRequest, Error> {
	log::trace!("getting container list");
	let container_info = client
		.get_container_images()
		.service_name(service_name.to_string())
		.send()
		.await?;

	if container_info.container_images.is_none() {
		Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;
	}
	log::trace!("recieved the container images");
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
	log::trace!("adding image to deployment container");
	let deployment_container = Container::builder()
		// image naming convention -> :service_name.label_name.
		.image(container_image_name)
		.ports("80".to_string(), ContainerServiceProtocol::Http)
		.build();

	// setting container health check config
	let health_check = ContainerServiceHealthCheckConfig::builder()
		.path("/".to_string())
		.build();

	// public endpoint request
	let public_endpoint_request = EndpointRequest::builder()
		.container_name(&service_name.to_string())
		.container_port(80)
		.set_health_check(Some(health_check))
		.build();

	// create deployment request
	let deployment_request = ContainerServiceDeploymentRequest::builder()
		.containers(service_name.to_string(), deployment_container)
		.set_public_endpoint(Some(public_endpoint_request))
		.build();
	log::trace!("deployment request created");
	Ok(deployment_request)
}

async fn get_default_url(
	service_name: &str,
	client: &lightsail::Client,
) -> Result<String, Error> {
	let container_service = client
		.get_container_services()
		.service_name(service_name.to_string())
		.send()
		.await?;

	if let Some(container_service) = container_service.container_services {
		let container_service = container_service.get(0);
		if let Some(container_service_info) = container_service {
			if let Some(url) = &container_service_info.url {
				return Ok(url.to_string());
			}
		}
	}

	Error::as_result()
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?
}
