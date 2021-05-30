use eve_rs::AsError;
use sqlx::Transaction;
use uuid::Uuid;

use crate::{
	db,
	error,
	models::{db_mapping::MachineType, rbac},
	utils::Error,
	Database,
};

pub async fn create_deployment_in_organisation(
	connection: &mut Transaction<'_, Database>,
	organisation_id: &[u8],
	name: &str,
	registry: &str,
	repository_id: Option<&str>,
	image_name: Option<&str>,
	image_tag: &str,
) -> Result<Uuid, Error> {
	// As of now, only our custom registry and docker hub is allowed
	match registry {
		"registry.docker.vicara.co" | "registry.hub.docker.com" => (),
		_ => {
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
		&organisation_id,
	)
	.await?;

	if registry == "registry.docker.vicara.co" {
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

pub async fn update_configuration_for_deployment(
	connection: &mut Transaction<'_, Database>,
	deployment_id: &[u8],
	exposed_ports: &[u16],
	environment_variables: &[(&str, &str)],
	persistent_volumes: &[(&str, &str)],
) -> Result<(), Error> {
	// check if deployment exists.
	db::get_deployment_by_id(connection, &deployment_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	// iterate over ports and add it to port table
	db::remove_all_exposed_ports_for_deployment(connection, deployment_id)
		.await?;
	for &port in exposed_ports {
		db::add_exposed_port_for_deployment(connection, deployment_id, port)
			.await?;
	}

	// iterate over env vars and add it to env vars table
	db::remove_all_environment_variables_for_deployment(
		connection,
		deployment_id,
	)
	.await?;
	for &(key, value) in environment_variables {
		db::add_environment_variable_for_deployment(
			connection,
			deployment_id,
			key,
			value,
		)
		.await?;
	}

	// iterate over persistent volumes and add it to the db
	db::remove_all_persistent_volumes_for_deployment(connection, deployment_id)
		.await?;
	for (name, path) in persistent_volumes {
		db::add_persistent_volume_for_deployment(
			connection,
			deployment_id,
			name,
			path,
		)
		.await?;
	}

	Ok(())
}

pub async fn create_deployment_upgrade_path_in_organisation(
	connection: &mut Transaction<'_, Database>,
	organisation_id: &[u8],
	name: &str,
	machine_types: &[MachineType],
	default_machine_type: &MachineType,
) -> Result<Uuid, Error> {
	db::get_deployment_upgrade_path_by_name_in_organisation(
		connection,
		name,
		organisation_id,
	)
	.await?
	.status(409)
	.body(error!(RESOURCE_EXISTS).to_string())?;

	let upgrade_path_uuid = db::generate_new_resource_id(connection).await?;
	let upgrade_path_id = upgrade_path_uuid.as_bytes();

	db::begin_deferred_constraints(connection).await?;
	db::create_resource(
		connection,
		upgrade_path_id,
		&format!("Deployment Upgrade Path: {}", name),
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(rbac::resource_types::DEPLOYMENT_UPGRADE_PATH)
			.unwrap(),
		organisation_id,
	)
	.await?;
	let default_machine_type_id = if let Some(id) =
		db::get_deployment_machine_type_id_for_configuration(
			connection,
			default_machine_type.cpu_count,
			default_machine_type.memory_count,
		)
		.await?
	{
		id
	} else {
		let machine_type_uuid =
			db::generate_new_deployment_machine_type_id(connection).await?;
		let machine_type_id = machine_type_uuid.as_bytes();
		db::add_deployment_machine_type(
			connection,
			machine_type_id,
			default_machine_type.cpu_count,
			default_machine_type.memory_count,
		)
		.await?;
		machine_type_id.to_vec()
	};
	db::create_deployment_upgrade_path(
		connection,
		upgrade_path_id,
		name,
		&default_machine_type_id,
	)
	.await?;

	// TODO sort machine_types

	// For each machine type, if a machine type already exists in the db, use
	// that. If it doesn't, insert a new one
	for machine_type in machine_types {
		let machine_type_id =
			db::get_deployment_machine_type_id_for_configuration(
				connection,
				machine_type.cpu_count,
				machine_type.memory_count,
			)
			.await?;
		let machine_type_id = if let Some(id) = machine_type_id {
			id
		} else {
			let machine_type_uuid =
				db::generate_new_deployment_machine_type_id(connection).await?;
			let machine_type_id = machine_type_uuid.as_bytes();
			db::add_deployment_machine_type(
				connection,
				machine_type_id,
				machine_type.cpu_count,
				machine_type.memory_count,
			)
			.await?;
			machine_type_id.to_vec()
		};

		db::add_deployment_machine_type_for_upgrade_path(
			connection,
			upgrade_path_id,
			&machine_type_id,
		)
		.await?;
	}
	db::end_deferred_constraints(connection).await?;

	Ok(upgrade_path_uuid)
}

pub async fn update_deployment_upgrade_path(
	connection: &mut Transaction<'_, Database>,
	upgrade_path_id: &[u8],
	name: &str,
	machine_types: &[MachineType],
) -> Result<(), Error> {
	let resource = db::get_resource_by_id(connection, &upgrade_path_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	db::get_deployment_upgrade_path_by_name_in_organisation(
		connection,
		name,
		&resource.owner_id,
	)
	.await?
	.status(409)
	.body(error!(RESOURCE_EXISTS).to_string())?;

	db::update_deployment_upgrade_path_name_by_id(
		connection,
		upgrade_path_id,
		name,
	)
	.await?;

	db::remove_all_machine_types_for_deployment_upgrade_path(
		connection,
		upgrade_path_id,
	)
	.await?;

	// For each machine type, if a machine type already exists in the db, use
	// that. If it doesn't, insert a new one
	for machine_type in machine_types {
		let machine_type_id =
			db::get_deployment_machine_type_id_for_configuration(
				connection,
				machine_type.cpu_count,
				machine_type.memory_count,
			)
			.await?;
		let machine_type_id = if let Some(id) = machine_type_id {
			id
		} else {
			let machine_type_uuid =
				db::generate_new_deployment_machine_type_id(connection).await?;
			let machine_type_id = machine_type_uuid.as_bytes();
			db::add_deployment_machine_type(
				connection,
				machine_type_id,
				machine_type.cpu_count,
				machine_type.memory_count,
			)
			.await?;
			machine_type_id.to_vec()
		};

		db::add_deployment_machine_type_for_upgrade_path(
			connection,
			upgrade_path_id,
			&machine_type_id,
		)
		.await?;
	}

	Ok(())
}

pub async fn create_deployment_entry_point_in_organisation(
	connection: &mut Transaction<'_, Database>,
	organisation_id: &[u8],
	sub_domain: &str,
	domain_id: &[u8],
	path: &str,
	entry_point_type: &str,
	deployment_id: Option<&[u8]>,
	deployment_port: Option<u16>,
	url: Option<&str>,
) -> Result<Uuid, Error> {
	db::get_deployment_entry_point_by_url(
		connection, sub_domain, domain_id, path,
	)
	.await?
	.status(409)
	.body(error!(RESOURCE_EXISTS).to_string())?;

	let entry_point_uuid = db::generate_new_resource_id(connection).await?;
	let entry_point_id = entry_point_uuid.as_bytes();

	let domain = db::get_organisation_domain_by_id(connection, domain_id)
		.await?
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	// Ensure that you can only make entry points to domains in your
	// organisation
	let domain_resource = db::get_resource_by_id(connection, domain_id)
		.await?
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;
	if domain_resource.owner_id != organisation_id {
		return Err(Error::empty()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string()));
	}

	db::create_resource(
		connection,
		entry_point_id,
		&format!(
			"Deployment entry point: {}.{}.{}",
			sub_domain, domain.name, path
		),
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(rbac::resource_types::DEPLOYMENT_ENTRY_POINT)
			.unwrap(),
		organisation_id,
	)
	.await?;

	match entry_point_type {
		"deployment" => {
			if let (Some(deployment_id), Some(deployment_port)) =
				(deployment_id, deployment_port)
			{
				db::add_deployment_entry_point_for_deployment(
					connection,
					entry_point_id,
					sub_domain,
					domain_id,
					path,
					deployment_id,
					deployment_port,
				)
				.await?;
			} else {
				return Err(Error::empty()
					.status(400)
					.body(error!(WRONG_PARAMETERS).to_string()));
			}
		}
		"redirect" => {
			if let Some(url) = url {
				db::add_deployment_entry_point_for_redirect(
					connection,
					entry_point_id,
					sub_domain,
					domain_id,
					path,
					url,
				)
				.await?;
			} else {
				return Err(Error::empty()
					.status(400)
					.body(error!(WRONG_PARAMETERS).to_string()));
			}
		}
		"proxy" => {
			if let Some(url) = url {
				db::add_deployment_entry_point_for_proxy(
					connection,
					entry_point_id,
					sub_domain,
					domain_id,
					path,
					url,
				)
				.await?;
			} else {
				return Err(Error::empty()
					.status(400)
					.body(error!(WRONG_PARAMETERS).to_string()));
			}
		}
		_ => {
			return Err(Error::empty()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string()))
		}
	}

	Ok(entry_point_uuid)
}

pub async fn update_deployment_entry_point(
	connection: &mut Transaction<'_, Database>,
	entry_point_id: &[u8],
	entry_point_type: &str,
	deployment_id: Option<&[u8]>,
	deployment_port: Option<u16>,
	url: Option<&str>,
) -> Result<(), Error> {
	db::get_deployment_entry_point_by_id(connection, entry_point_id)
		.await?
		.status(409)
		.body(error!(RESOURCE_EXISTS).to_string())?;

	match entry_point_type {
		"deployment" => {
			if let (Some(deployment_id), Some(deployment_port)) =
				(deployment_id, deployment_port)
			{
				db::update_deployment_entry_point_to_deployment(
					connection,
					entry_point_id,
					deployment_id,
					deployment_port,
				)
				.await?;
			} else {
				return Err(Error::empty()
					.status(400)
					.body(error!(WRONG_PARAMETERS).to_string()));
			}
		}
		"redirect" => {
			if let Some(url) = url {
				db::update_deployment_entry_point_to_redirect(
					connection,
					entry_point_id,
					url,
				)
				.await?;
			} else {
				return Err(Error::empty()
					.status(400)
					.body(error!(WRONG_PARAMETERS).to_string()));
			}
		}
		"proxy" => {
			if let Some(url) = url {
				db::update_deployment_entry_point_to_proxy(
					connection,
					entry_point_id,
					url,
				)
				.await?;
			} else {
				return Err(Error::empty()
					.status(400)
					.body(error!(WRONG_PARAMETERS).to_string()));
			}
		}
		_ => {
			return Err(Error::empty()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string()))
		}
	}

	Ok(())
}
