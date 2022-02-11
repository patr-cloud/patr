use std::{collections::BTreeMap, str};

use api_models::{
	models::workspace::infrastructure::deployment::{
		Deployment, DeploymentRegistry, DeploymentRunningDetails,
		DeploymentStatus, EnvironmentVariableValue, ExposedPortType,
		PatrRegistry,
	},
	utils::{constants, StringifiedU16, Uuid},
};
use eve_rs::AsError;
use lapin::{options::BasicPublishOptions, BasicProperties};

use crate::{
	db, error,
	models::{
		rabbitmq::{
			DeploymentRequestData, RequestData, RequestMessage, RequestType,
		},
		rbac,
	},
	service::{self, infrastructure::kubernetes},
	utils::{get_current_time_millis, settings::Settings, validator, Error},
	Database,
};

/// # Description
/// This function creates a deployment under an workspace account
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `workspace_id` -  an unsigned 8 bit integer array containing the id of
///   workspace
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
pub async fn create_deployment_in_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	name: &str,
	registry: &DeploymentRegistry,
	image_tag: &str,
	region: &Uuid,
	machine_type: &Uuid,
	deploy_on_push: bool,
	min_horizontal_scale: u16,
	max_horizontal_scale: u16,
	ports: &BTreeMap<u16, ExposedPortType>,
	environment_variables: &BTreeMap<String, EnvironmentVariableValue>,
	request_id: &Uuid,
) -> Result<Uuid, Error> {
	// As of now, only our custom registry is allowed
	// Docker hub will also be allowed in the near future
	log::trace!("request_id: {} - Checking if the deployment's image is in patr registry", request_id);
	if !registry.is_patr_registry() {
		return Err(Error::empty()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string()));
	}

	if image_tag.is_empty() {
		return Err(Error::empty()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string()));
	}

	// validate deployment name
	log::trace!("request_id: {} - Validating deployment name", request_id);
	if !validator::is_deployment_name_valid(name) {
		return Err(Error::empty()
			.status(200)
			.body(error!(INVALID_DEPLOYMENT_NAME).to_string()));
	}

	log::trace!(
		"request_id: {} - Checking if the deployment name already exists",
		request_id
	);
	let existing_deployment =
		db::get_deployment_by_name_in_workspace(connection, name, workspace_id)
			.await?;
	if existing_deployment.is_some() {
		Error::as_result()
			.status(200)
			.body(error!(RESOURCE_EXISTS).to_string())?;
	}

	log::trace!("request_id: {} - Generating new resource id", request_id);
	let deployment_id = db::generate_new_resource_id(connection).await?;

	db::create_resource(
		connection,
		&deployment_id,
		&format!("Deployment: {}", name),
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(rbac::resource_types::DEPLOYMENT)
			.unwrap(),
		workspace_id,
		get_current_time_millis(),
	)
	.await?;
	log::trace!("request_id: {} - Created resource", request_id);

	match registry {
		DeploymentRegistry::PatrRegistry {
			registry: _,
			repository_id,
		} => {
			log::trace!("request_id: {} - Creating database record with internal registry", request_id);
			db::create_deployment_with_internal_registry(
				connection,
				&deployment_id,
				name,
				repository_id,
				image_tag,
				workspace_id,
				region,
				machine_type,
				deploy_on_push,
				min_horizontal_scale,
				max_horizontal_scale,
			)
			.await?;
		}
		DeploymentRegistry::ExternalRegistry {
			registry,
			image_name,
		} => {
			log::trace!("request_id: {} - Creating database record with external registry", request_id);
			db::create_deployment_with_external_registry(
				connection,
				&deployment_id,
				name,
				registry,
				image_name,
				image_tag,
				workspace_id,
				region,
				machine_type,
				deploy_on_push,
				min_horizontal_scale,
				max_horizontal_scale,
			)
			.await?;
		}
	}

	for (port, port_type) in ports {
		log::trace!(
			"request_id: {} - Adding exposed port entry to database",
			request_id
		);
		db::add_exposed_port_for_deployment(
			connection,
			&deployment_id,
			*port,
			port_type,
		)
		.await?;
	}

	for (key, value) in environment_variables {
		log::trace!(
			"request_id: {} - Adding environment variable entry to database",
			request_id
		);
		db::add_environment_variable_for_deployment(
			connection,
			&deployment_id,
			key,
			if let EnvironmentVariableValue::String(value) = value {
				value
			} else {
				return Err(Error::empty()
					.status(400)
					.body(error!(WRONG_PARAMETERS).to_string()));
			},
		)
		.await?;
	}

	// TODO UPDATE ENTRY POINTS

	Ok(deployment_id)
}

pub async fn start_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!(
		"request_id: {} - Starting deployment with id: {}",
		request_id,
		deployment_id
	);
	let (deployment, workspace_id, full_image, running_details) =
		service::get_full_deployment_config(
			connection,
			deployment_id,
			request_id,
		)
		.await?;

	log::trace!(
		"request_id: {} - Updating kubernetes deployment",
		request_id
	);

	db::update_deployment_status(
		connection,
		deployment_id,
		&DeploymentStatus::Deploying,
	)
	.await?;

	let channel = &service::get_app().rabbit_mq.channel_a;

	let content = RequestMessage {
		request_type: RequestType::Update,
		request_data: RequestData::Deployment(Box::new(
			DeploymentRequestData::Update {
				workspace_id,
				deployment,
				full_image,
				running_details,
				config: Box::new(config.clone()),
				request_id: request_id.clone(),
			},
		)),
	};

	channel
		.basic_publish(
			"",
			"infrastructure",
			BasicPublishOptions::default(),
			serde_json::to_string(&content)?.as_bytes(),
			BasicProperties::default(),
		)
		.await?
		.await?;

	Ok(())
}

pub async fn stop_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!(
		"Stopping the deployment with id: {} and request_id: {}",
		deployment_id,
		request_id
	);
	log::trace!("request_id: {} - Getting deployment id from db", request_id);
	let deployment = db::get_deployment_by_id(connection, deployment_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	log::trace!(
		"request_id: {} - deleting the deployment from digitalocean kubernetes",
		request_id
	);

	let channel = &service::get_app().rabbit_mq.channel_a;

	let content = RequestMessage {
		request_type: RequestType::Update,
		request_data: RequestData::Deployment(Box::new(
			DeploymentRequestData::Delete {
				workspace_id: deployment.workspace_id,
				deployment_id: deployment_id.clone(),
				config: config.clone(),
				request_id: request_id.clone(),
			},
		)),
	};

	channel
		.basic_publish(
			"",
			"infrastructure",
			BasicPublishOptions::default(),
			serde_json::to_string(&content)?.as_bytes(),
			BasicProperties::default(),
		)
		.await?
		.await?;

	// TODO: implement logic for handling domains of the stopped deployment
	log::trace!("request_id: {} - Updating deployment status", request_id);
	db::update_deployment_status(
		connection,
		deployment_id,
		&DeploymentStatus::Stopped,
	)
	.await?;

	Ok(())
}

pub async fn delete_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!(
		"request_id: {} - Deleting the deployment with id: {}",
		request_id,
		deployment_id
	);

	let deployment = db::get_deployment_by_id(connection, deployment_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	log::trace!("request_id: {} - Stopping the deployment", request_id);
	service::stop_deployment(connection, deployment_id, config, request_id)
		.await?;

	log::trace!(
		"request_id: {} - Updating the deployment name in the database",
		request_id
	);
	db::update_deployment_name(
		connection,
		deployment_id,
		&format!("patr-deleted: {}-{}", deployment.name, deployment_id),
	)
	.await?;

	log::trace!("request_id: {} - Updating deployment status", request_id);
	db::update_deployment_status(
		connection,
		deployment_id,
		&DeploymentStatus::Deleted,
	)
	.await?;

	Ok(())
}

pub async fn get_deployment_container_logs(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<String, Error> {
	log::trace!(
		"Getting deployment logs for deployment_id: {} with request_id: {}",
		deployment_id,
		request_id
	);

	let deployment = db::get_deployment_by_id(connection, deployment_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let logs = kubernetes::get_container_logs(
		&deployment.workspace_id,
		deployment_id,
		config,
		request_id,
	)
	.await?;
	log::trace!("request_id: {} - Logs retreived successfully", request_id);

	Ok(logs)
}

pub async fn update_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
	name: Option<&str>,
	region: Option<&Uuid>,
	machine_type: Option<&Uuid>,
	deploy_on_push: Option<bool>,
	min_horizontal_scale: Option<u16>,
	max_horizontal_scale: Option<u16>,
	ports: Option<&BTreeMap<u16, ExposedPortType>>,
	environment_variables: Option<&BTreeMap<String, EnvironmentVariableValue>>,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!(
		"request_id: {} - Updating deployment with id: {}",
		request_id,
		deployment_id
	);
	db::update_deployment_details(
		connection,
		deployment_id,
		name,
		region,
		machine_type,
		deploy_on_push,
		min_horizontal_scale,
		max_horizontal_scale,
	)
	.await?;

	if let Some(ports) = ports {
		log::trace!(
			"request_id: {} - Updating deployment ports in the database",
			request_id
		);
		db::remove_all_exposed_ports_for_deployment(connection, deployment_id)
			.await?;
		for (port, exposed_port_type) in ports {
			db::add_exposed_port_for_deployment(
				connection,
				deployment_id,
				*port,
				exposed_port_type,
			)
			.await?;
		}
	}

	if let Some(environment_variables) = environment_variables {
		log::trace!(
			"request_id: {} - Updating deployment environment variables in the database",
			request_id
		);
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
				if let EnvironmentVariableValue::String(value) = value {
					value
				} else {
					return Err(Error::empty()
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string()));
				},
			)
			.await?;
		}
	}
	log::trace!(
		"request_id: {} - Deployment updated in the database",
		request_id
	);

	Ok(())
}

pub async fn get_full_deployment_config(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
	request_id: &Uuid,
) -> Result<(Deployment, Uuid, String, DeploymentRunningDetails), Error> {
	log::trace!(
		"request_id: {} - Getting the full deployment config for deployment with id: {}",
		request_id,
		deployment_id
	);
	let (
		deployment,
		workspace_id,
		deploy_on_push,
		min_horizontal_scale,
		max_horizontal_scale,
	) = db::get_deployment_by_id(connection, deployment_id)
		.await?
		.and_then(|deployment| {
			Some((
				Deployment {
					id: deployment.id,
					name: deployment.name,
					registry: if deployment.registry == constants::PATR_REGISTRY
					{
						DeploymentRegistry::PatrRegistry {
							registry: PatrRegistry,
							repository_id: deployment.repository_id?,
						}
					} else {
						DeploymentRegistry::ExternalRegistry {
							registry: deployment.registry,
							image_name: deployment.image_name?,
						}
					},
					image_tag: deployment.image_tag,
					status: deployment.status,
					region: deployment.region,
					machine_type: deployment.machine_type,
				},
				deployment.workspace_id,
				deployment.deploy_on_push,
				deployment.min_horizontal_scale as u16,
				deployment.max_horizontal_scale as u16,
			))
		})
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let full_image = match &deployment.registry {
		DeploymentRegistry::PatrRegistry {
			registry: _,
			repository_id,
		} => {
			let repository =
				db::get_docker_repository_by_id(connection, repository_id)
					.await?
					.status(404)
					.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;
			let workspace =
				db::get_workspace_info(connection, &repository.workspace_id)
					.await?
					.status(500)?;
			format!(
				"{}/{}/{}",
				constants::PATR_REGISTRY,
				workspace.name,
				repository.name
			)
		}
		DeploymentRegistry::ExternalRegistry {
			registry,
			image_name,
		} => {
			format!("{}/{}", registry, image_name)
		}
	};

	let ports = db::get_exposed_ports_for_deployment(connection, deployment_id)
		.await?
		.into_iter()
		.map(|(port, port_type)| (StringifiedU16::new(port), port_type))
		.collect();

	let environment_variables =
		db::get_environment_variables_for_deployment(connection, deployment_id)
			.await?
			.into_iter()
			.map(|(key, value)| (key, EnvironmentVariableValue::String(value)))
			.collect();
	log::trace!("request_id: {} - Full deployment config for deployment with id: {} successfully retreived", request_id, deployment_id);

	Ok((
		deployment,
		workspace_id,
		full_image,
		DeploymentRunningDetails {
			deploy_on_push,
			min_horizontal_scale,
			max_horizontal_scale,
			ports,
			environment_variables,
		},
	))
}
