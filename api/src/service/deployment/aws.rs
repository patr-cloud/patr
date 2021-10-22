use std::{ops::DerefMut, process::Stdio, time::Duration};

use eve_rs::AsError;
use lightsail::{
	model::{
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
use uuid::Uuid;

use crate::{
	db,
	error,
	models::db_mapping::{
		CloudPlatform,
		DeploymentMachineType,
		DeploymentStatus,
		ManagedDatabaseEngine,
		ManagedDatabasePlan,
		ManagedDatabaseStatus,
	},
	service::{
		self,
		deployment::{deployment, managed_database},
	},
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn deploy_container(
	image_id: String,
	region: String,
	deployment_id: Vec<u8>,
	config: Settings,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();
	log::trace!("Deploying the container with id: {} and image: {} on Aws Lightsail with request_id: {}",
		hex::encode(&deployment_id),
		image_id,
		request_id
	);
	let deployment = db::get_deployment_by_id(
		service::get_app().database.acquire().await?.deref_mut(),
		&deployment_id,
	)
	.await?
	.status(500)
	.body(error!(SERVER_ERROR).to_string())?;

	let deployment_id_string = hex::encode(&deployment_id);
	log::trace!(
		"request_id: {} - Deploying deployment: {}",
		request_id,
		deployment_id_string
	);
	let _ = super::update_deployment_status(
		&deployment_id,
		&DeploymentStatus::Pushed,
	)
	.await;

	log::trace!("request_id: {} - Pulling image from registry", request_id);
	deployment::pull_image_from_registry(&image_id, &config).await?;
	log::trace!("request_id: {} - Image pulled", request_id);

	// new name for the docker image
	let new_repo_name = format!("patr-cloud/{}", deployment_id_string);

	log::trace!("request_id: {} - Pushing to {}", new_repo_name, request_id);

	// rename the docker image with the digital ocean registry url
	deployment::tag_docker_image(&image_id, &new_repo_name).await?;
	log::trace!("request_id: {} - Image tagged", request_id);

	// Get credentails for aws lightsail
	let client = get_lightsail_client(&region);

	let label_name = "latest".to_string();

	let _ = super::update_deployment_status(
		&deployment_id,
		&DeploymentStatus::Deploying,
	)
	.await;

	let app_exists =
		get_app_default_url(&deployment_id_string, &region).await?;
	let default_url = if let Some(default_url) = app_exists {
		push_image_to_lightsail(
			&deployment_id_string,
			&new_repo_name,
			&label_name,
			&region,
			request_id,
		)
		.await?;
		log::trace!(
			"request_id: {} - pushed the container into aws registry",
			request_id
		);
		default_url.replace("https://", "").replace("/", "")
	} else {
		// create container service
		log::trace!(
			"request_id: {} - creating new container service",
			request_id
		);
		create_container_service(
			&deployment_id_string,
			&label_name,
			&new_repo_name,
			&region,
			deployment.horizontal_scale,
			&deployment.machine_type,
			&client,
			request_id,
		)
		.await?
	};

	log::trace!("request_id: {} - Creating container deployment", request_id);
	deploy_application(
		&deployment_id,
		&deployment_id_string,
		&client,
		request_id,
	)
	.await?;
	log::trace!("request_id: {} - App deployed", request_id);

	// wait for the app to be completed to be deployed
	log::trace!(
		"request_id: {} - default url is {}",
		default_url,
		request_id
	);
	log::trace!("request_id: {} - checking deployment status", request_id);
	wait_for_deployment(&deployment_id_string, &client).await?;

	// update DNS
	log::trace!("request_id: {} - updating DNS", request_id);
	super::add_cname_record(
		&deployment_id_string,
		"nginx.patr.cloud",
		&config,
		false,
	)
	.await?;
	log::trace!("request_id: {} - DNS Updated", request_id);

	log::trace!("request_id: {} - adding reverse proxy", request_id);
	deployment::update_nginx_with_all_domains_for_deployment(
		&deployment_id_string,
		&default_url,
		deployment.domain_name.as_deref(),
		&config,
		request_id,
	)
	.await?;

	let _ = super::update_deployment_status(
		&deployment_id,
		&DeploymentStatus::Running,
	)
	.await;
	log::trace!(
		"request_id: {} - deleting image tagged with patr-cloud",
		request_id
	);
	let _ = super::delete_docker_image(&new_repo_name).await;
	log::trace!("request_id: {} - deleting the pulled image", request_id);
	let _ = super::delete_docker_image(&image_id).await;
	log::trace!("request_id: {} - Docker image deleted", request_id);

	Ok(())
}

pub(super) async fn delete_deployment(
	_connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
	region: &str,
	_config: &Settings,
	request_id: Uuid,
) -> Result<(), Error> {
	// Get credentails for aws lightsail
	log::trace!(
		"request_id: {} - getting credentials from lightsail",
		request_id
	);
	let client = get_lightsail_client(region);
	let deployment_id_string = hex::encode(deployment_id);

	// certificate needs to be detached inorder to get deleted but there is no
	// endpoint to detach the certificate
	log::trace!("request_id: {} - deleting deployment", request_id);

	client
		.delete_container_service()
		.service_name(&deployment_id_string)
		.send()
		.await
		.map_err(|err| {
			log::error!("Error during deletion of service, {}", err);
			err
		})?;
	log::trace!(
		"request_id: {} - deployment deleted successfully!",
		request_id
	);

	Ok(())
}

pub(super) async fn get_container_logs(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
	_config: &Settings,
	request_id: Uuid,
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
	log::trace!(
		"request_id: {} - getting credentails from aws lightsail",
		request_id
	);
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
	let request_id = Uuid::new_v4();
	log::trace!("Creating a managed database on aws lightsail with id: {} and db_name: {} with request_id: {}",
		hex::encode(&database_id),
		db_name,
		request_id
	);
	let client = get_lightsail_client(region);

	let username = "patr_admin".to_string();
	let password = thread_rng()
		.sample_iter(&Alphanumeric)
		.take(8)
		.map(char::from)
		.collect::<String>();

	log::trace!(
		"request_id: {} - sending the create db cluster request to aws",
		request_id
	);
	client
		.create_relational_database()
		.master_database_name(db_name)
		.master_username(&username)
		.master_user_password(&password)
		.publicly_accessible(true)
		.relational_database_blueprint_id(format!(
			"{}_{}",
			engine,
			match version {
				"8" => "8_0",
				value => value,
			}
		))
		.relational_database_bundle_id(database_plan.as_aws_plan()?)
		.relational_database_name(hex::encode(database_id))
		.send()
		.await?;
	log::trace!("request_id: {} - database created", request_id);

	let database_id = database_id.to_vec();
	let region = region.to_string();

	task::spawn(async move {
		let result = update_database_cluster_credentials(
			database_id.clone(),
			region,
			username,
			password,
			request_id,
		)
		.await;

		if let Err(error) = result {
			let _ = managed_database::update_managed_database_status(
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
	let request_id = Uuid::new_v4();
	log::trace!("Deleting managed database on Awl lightsail with digital_ocean_id: {} and request_id: {}",
		hex::encode(database_id),
		request_id,
	);

	log::trace!("request_id: {} - getting lightsail client", request_id);
	let client = get_lightsail_client(region);

	log::trace!(
		"request_id: {} - getting database info from lightsail",
		request_id
	);
	let database_cluster = client
		.get_relational_database()
		.relational_database_name(hex::encode(database_id))
		.send()
		.await;

	if database_cluster.is_err() {
		return Ok(());
	}

	log::trace!(
		"request_id: {} - deleting database from lightsail",
		request_id
	);
	client
		.delete_relational_database()
		.relational_database_name(hex::encode(database_id))
		.send()
		.await?;

	Ok(())
}

pub(super) async fn get_app_default_url(
	deployment_id: &str,
	region: &str,
) -> Result<Option<String>, Error> {
	let client = get_lightsail_client(region);
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

async fn update_database_cluster_credentials(
	database_id: Vec<u8>,
	region: String,
	username: String,
	password: String,
	request_id: Uuid,
) -> Result<(), Error> {
	let client = get_lightsail_client(&region);

	log::trace!(
		"request_id: {} - getting database info from lightsail",
		request_id
	);
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

		log::trace!("request_id: {} - checking the database state", request_id);
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

	log::trace!(
		"request_id: {} updating managed database credentials",
		request_id
	);
	managed_database::update_managed_database_credentials_for_database(
		&database_id,
		&host,
		port,
		&username,
		&password,
	)
	.await?;

	log::trace!(
		"request_id: {} - updating to the db status to running",
		request_id
	);
	// wait for database to start
	super::update_managed_database_status(
		&database_id,
		&ManagedDatabaseStatus::Running,
	)
	.await?;
	log::trace!("request_id: {} - database successfully updated", request_id);

	Ok(())
}

async fn create_container_service(
	deployment_id: &str,
	label_name: &str,
	new_repo_name: &str,
	region: &str,
	horizontal_scale: i16,
	machine_type: &DeploymentMachineType,
	client: &lightsail::Client,
	request_id: Uuid,
) -> Result<String, Error> {
	log::trace!("request_id: {} - checking if the service exists or is in the process of getting deleted", request_id);
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
	log::trace!("request_id: {} - container service created", request_id);

	push_image_to_lightsail(
		deployment_id,
		new_repo_name,
		label_name,
		region,
		request_id,
	)
	.await?;
	log::trace!(
		"request_id: {} - pushed the container into aws registry",
		request_id
	);

	let default_url = created_service
		.container_service
		.map(|service| service.url)
		.flatten()
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;
	Ok(default_url.replace("https://", "").replace("/", ""))
}

async fn deploy_application(
	deployment_id: &[u8],
	deployment_id_string: &str,
	client: &lightsail::Client,
	request_id: Uuid,
) -> Result<(), Error> {
	let container_image_name =
		get_latest_image_name(deployment_id_string, client, request_id).await?;
	log::trace!(
		"request_id: {} - adding image to deployment container",
		request_id
	);

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
	log::trace!("request_id: {} - deployment request created", request_id);

	let _ = client
		.create_container_service_deployment()
		.set_containers(deployment_request.containers)
		.set_service_name(Some(deployment_id_string.to_string()))
		.set_public_endpoint(deployment_request.public_endpoint)
		.send()
		.await?;
	log::trace!(
		"request_id: {} - created the deployment successfully",
		request_id
	);

	Ok(())
}

async fn get_latest_image_name(
	deployment_id: &str,
	client: &lightsail::Client,
	request_id: Uuid,
) -> Result<String, Error> {
	log::trace!("request_id: {} - getting container list", request_id);
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
	log::trace!("request_id: {} - recieved the container images", request_id);

	Ok(container_image)
}

async fn push_image_to_lightsail(
	deployment_id: &str,
	new_repo_name: &str,
	label_name: &str,
	region: &str,
	request_id: Uuid,
) -> Result<(), Error> {
	log::trace!("request_id: {} - pushing image to lightsail", request_id);
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
		} else if let Some(ContainerServiceDeploymentState::Failed) =
			deployment_state
		{
			return Err(Error::empty()
				.status(500)
				.body(error!(SERVER_ERROR).to_string()));
		}
		time::sleep(Duration::from_millis(1000)).await;
	}
}

fn get_lightsail_client(region: &str) -> lightsail::Client {
	let deployment_region = lightsail::Region::new(region.to_string());
	let client_builder = lightsail::Config::builder()
		.region(Some(deployment_region))
		.build();
	lightsail::Client::from_conf(client_builder)
}
