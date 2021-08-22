mod digitalocean;

pub use digitalocean::*;
use eve_rs::AsError;
use uuid::Uuid;

use crate::{
	db,
	error,
	models::rbac,
	utils::{get_current_time_millis, Error},
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
	} else {
		return Err(Error::empty()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string()));
	}

	Ok(deployment_uuid)
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
