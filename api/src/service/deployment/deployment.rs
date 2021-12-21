use std::{collections::BTreeMap, str};

use api_models::{
	models::workspace::infrastructure::deployment::{
		Deployment,
		DeploymentRegistry,
		DeploymentRunningDetails,
		DeploymentStatus,
		EntryPointMapping,
		EnvironmentVariableValue,
		ExposedPortType,
		PatrRegistry,
	},
	utils::{constants, Uuid},
};
use eve_rs::AsError;

use crate::{
	db,
	error,
	models::rbac,
	service::{self, deployment::kubernetes},
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
	urls: &[EntryPointMapping],
) -> Result<Uuid, Error> {
	// As of now, only our custom registry is allowed
	// Docker hub will also be allowed in the near future
	if !registry.is_patr_registry() {
		return Err(Error::empty()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string()));
	}

	// validate deployment name
	if !validator::is_deployment_name_valid(name) {
		return Err(Error::empty()
			.status(200)
			.body(error!(INVALID_DEPLOYMENT_NAME).to_string()));
	}

	let existing_deployment =
		db::get_deployment_by_name_in_workspace(connection, name, workspace_id)
			.await?;
	if existing_deployment.is_some() {
		Error::as_result()
			.status(200)
			.body(error!(RESOURCE_EXISTS).to_string())?;
	}

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

	match registry {
		DeploymentRegistry::PatrRegistry {
			registry: _,
			repository_id,
		} => {
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
		db::add_exposed_port_for_deployment(
			connection,
			&deployment_id,
			*port,
			port_type,
		)
		.await?;
	}

	for (key, value) in environment_variables {
		db::add_environment_variable_for_deployment(
			connection,
			&deployment_id,
			key,
			&if let EnvironmentVariableValue::String(value) = value {
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
) -> Result<(), Error> {
	let deployment = db::get_deployment_by_id(connection, deployment_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let image_id = deployment.get_full_image(connection).await?;

	let request_id = Uuid::new_v4();
	log::trace!(
		"Deploying: {} and image: {} Kubernetes with request_id: {}",
		deployment_id,
		image_id,
		request_id
	);

	kubernetes::update_kubernetes_deployment(connection, deployment_id, config)
		.await?;

	Ok(())
}

pub async fn stop_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
	config: &Settings,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();
	log::trace!(
		"Stopping the deployment with id: {} and request_id: {}",
		deployment_id,
		request_id
	);
	log::trace!("request_id: {} - Getting deployment id from db", request_id);
	let _ = db::get_deployment_by_id(connection, deployment_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	log::trace!(
		"request_id: {} - deleting the deployment from digitalocean kubernetes",
		request_id
	);
	kubernetes::delete_kubernetes_deployment(
		deployment_id,
		config,
		&request_id,
	)
	.await?;

	// TODO: implement logic for handling domains of the stopped deployment

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
) -> Result<(), Error> {
	let deployment = db::get_deployment_by_id(connection, deployment_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	service::stop_deployment(connection, deployment_id, config).await?;

	db::update_deployment_name(
		connection,
		deployment_id,
		&format!("patr-deleted: {}-{}", deployment.name, deployment_id),
	)
	.await?;

	// TODO: implement logic for handling domains of the deleted deployment

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
) -> Result<String, Error> {
	let request_id = Uuid::new_v4();
	log::trace!(
		"Getting deployment logs for deployment_id: {} with request_id: {}",
		deployment_id,
		request_id
	);

	let _ = db::get_deployment_by_id(connection, deployment_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let logs =
		kubernetes::get_container_logs(deployment_id, request_id, config)
			.await?;

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
	_urls: Option<&[EntryPointMapping]>,
	config: &Settings,
) -> Result<(), Error> {
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

	// TODO entry points

	Ok(())
}

#[allow(dead_code)]
async fn check_if_image_exists_in_registry(
	_connection: &mut <Database as sqlx::Database>::Connection,
	_image_id: &str,
) -> Result<bool, Error> {
	// TODO: fill this function for checking if the user has pushed the image
	// before making the deployment if the user has pushed the image then return
	// true
	Ok(false)
}
