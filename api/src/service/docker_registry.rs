use std::{collections::BTreeMap, str::FromStr};

use api_models::{
	models::workspace::infrastructure::deployment::ExposedPortType,
	utils::{DateTime, StringifiedU16, Uuid},
};
use chrono::Utc;
use eve_rs::AsError;

use crate::{
	db,
	error,
	models::{
		rbac,
		DockerRepositoryManifest,
		RegistryToken,
		RegistryTokenAccess,
		V1Compatibility,
	},
	utils::{constants::free_limits, settings::Settings, Error},
	Database,
};

pub async fn delete_docker_repository_image(
	connection: &mut <Database as sqlx::Database>::Connection,
	repository_id: &Uuid,
	digest: &str,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!(
		"request_id: {} - Deleting docker repository image with digest: {}",
		request_id,
		digest
	);
	let repository = db::get_docker_repository_by_id(connection, repository_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let repo_name = format!("{}/{}", repository.workspace_id, repository.name);

	// First, delete all tags for the given image
	log::trace!(
		"request_id: {} - Deleting all tags for the given image.",
		request_id
	);
	let tags = db::get_tags_for_docker_repository_image(
		connection,
		repository_id,
		digest,
	)
	.await?;
	for tag in tags {
		db::delete_tag_from_docker_repository(
			connection,
			repository_id,
			&tag.tag,
		)
		.await?;
	}

	log::trace!("request_id: {} - Deleting docker repository image with digest: {} from the database", request_id, digest);
	db::delete_docker_repository_image(connection, repository_id, digest)
		.await?;

	let total_storage =
		db::get_total_size_of_docker_repositories_for_workspace(
			connection,
			&repository.workspace_id,
		)
		.await?;
	db::update_docker_repo_usage_history(
		connection,
		&repository.workspace_id,
		&(total_storage as i64),
		&DateTime::from(Utc::now()),
	)
	.await?;

	delete_docker_repository_image_in_registry(
		connection, &repo_name, digest, config, request_id,
	)
	.await?;

	Ok(())
}

pub async fn delete_docker_repository_image_in_registry(
	connection: &mut sqlx::PgConnection,
	repo_name: &str,
	digest: &str,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), eve_rs::Error<()>> {
	let god_user =
		db::get_user_by_user_id(connection, rbac::GOD_USER_ID.get().unwrap())
			.await?
			.unwrap();

	log::trace!("request_id: {} - Deleting docker repository image with digest: {} from the registry", request_id, digest);
	let response = reqwest::Client::new()
		.delete(format!(
			"{}://{}/v2/{}/manifests/{}",
			if config.docker_registry.registry_url.starts_with("localhost") {
				"http"
			} else {
				"https"
			},
			config.docker_registry.registry_url,
			repo_name,
			digest
		))
		.bearer_auth(
			RegistryToken::new(
				config.docker_registry.issuer.clone(),
				Utc::now(),
				god_user.username.clone(),
				config,
				vec![RegistryTokenAccess {
					r#type: "repository".to_string(),
					name: repo_name.to_owned(),
					actions: vec!["delete".to_string()],
				}],
			)
			.to_string(
				config.docker_registry.private_key.as_ref(),
				config.docker_registry.public_key_der.as_ref(),
			)?,
		)
		.header(
			reqwest::header::ACCEPT,
			format!(
				"{}, {}",
				"application/vnd.docker.distribution.manifest.v2+json",
				"application/vnd.oci.image.manifest.v1+json"
			),
		)
		.send()
		.await?;

	// https://docs.docker.com/registry/spec/api/#delete-manifest
	// 200 => Accepted (Success)
	// 400 => Invalid Name or Reference
	// 404 => No Such Repository Error

	let response_status = response.status();
	if response_status.is_success() {
		log::trace!("request_id: {} - Deleting docker repository image with digest: {} from the registry was successful", request_id, digest);
		Ok(())
	} else {
		let response_msg = response.text().await?;
		log::trace!("request_id: {} - Deleting docker repository image with digest: {} failed with response {}", request_id, digest, response_msg);

		if response_status == 404 || response_status == 400 {
			log::warn!("request_id: {} - Since image is not there considering it as already deleted", request_id);
			Ok(())
		} else {
			Err(Error::empty())
		}
	}
}

pub async fn delete_docker_repository(
	connection: &mut <Database as sqlx::Database>::Connection,
	repository_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!(
		"request_id: {} - Deleting docker repository with id: {}",
		request_id,
		repository_id
	);
	let repository = db::get_docker_repository_by_id(connection, repository_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let repo_name = format!("{}/{}", &repository.workspace_id, repository.name);

	let images = db::get_list_of_digests_for_docker_repository(
		connection,
		repository_id,
	)
	.await?;

	log::trace!(
		"request_id: {} - Deleting all tags for the given repository.",
		request_id
	);
	db::delete_all_tags_for_docker_repository(connection, repository_id)
		.await?;
	log::trace!(
		"request_id: {} - Deleting all images for the given repository",
		request_id
	);
	db::delete_all_images_for_docker_repository(connection, repository_id)
		.await?;

	log::trace!(
		"request_id: {} - Updating the name of docker repository",
		request_id
	);
	db::delete_docker_repository(connection, repository_id, &Utc::now())
		.await?;

	log::trace!("request_id: {} - Deleting docker images of the repositories from the registry", request_id);
	for image in images {
		delete_docker_repository_image_in_registry(
			connection,
			&repo_name,
			&image.digest,
			config,
			request_id,
		)
		.await?;
	}

	log::trace!("request_id: {} - Deleting docker repository from the registry was successful", request_id);
	Ok(())
}

pub async fn get_exposed_port_for_docker_image(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
	repository_name: &str,
	tag: &str,
) -> Result<BTreeMap<StringifiedU16, ExposedPortType>, Error> {
	let god_user =
		db::get_user_by_user_id(connection, rbac::GOD_USER_ID.get().unwrap())
			.await?
			.unwrap();

	let exposed_ports = reqwest::Client::new()
		.get(format!(
			"{}://{}/v2/{}/manifests/{}",
			if config.docker_registry.registry_url.starts_with("localhost") {
				"http"
			} else {
				"https"
			},
			config.docker_registry.registry_url,
			&repository_name,
			tag
		))
		.bearer_auth(
			RegistryToken::new(
				config.docker_registry.issuer.clone(),
				Utc::now(),
				god_user.username.clone(),
				config,
				vec![RegistryTokenAccess {
					r#type: "repository".to_string(),
					name: repository_name.to_string(),
					actions: vec!["pull".to_string()],
				}],
			)
			.to_string(
				config.docker_registry.private_key.as_ref(),
				config.docker_registry.public_key_der.as_ref(),
			)?,
		)
		.header(
			reqwest::header::CONTENT_TYPE,
			"application/vnd.docker.distribution.manifest.v1+prettyjws",
		)
		.send()
		.await?
		.json::<DockerRepositoryManifest>()
		.await
		.map_err(|e| {
			log::error!("Error while parsing manifest json - {}", e);
			e
		})?
		.history
		.into_iter()
		.filter_map(|v1_comp_str| {
			serde_json::from_str::<V1Compatibility>(
				&v1_comp_str.v1_compatibility,
			)
			.ok()
		})
		.filter_map(|v1_comp| v1_comp.container_config.exposed_ports)
		.flat_map(IntoIterator::into_iter)
		.map(|(port, _)| port)
		.flat_map(|port| {
			if let Some((port, "tcp")) = port.split_once('/') {
				Some((
					StringifiedU16::from_str(port).ok()?,
					ExposedPortType::Http,
				))
			} else {
				None
			}
		})
		.collect::<BTreeMap<_, _>>();

	Ok(exposed_ports)
}

pub async fn docker_repo_storage_limit_crossed(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	additional_bytes: usize,
) -> Result<bool, Error> {
	let card_added =
		db::get_default_payment_method_for_workspace(connection, workspace_id)
			.await?
			.is_some();
	if card_added {
		// card added, so user is charged based on storage
		return Ok(false);
	}

	let total_usage_so_far_in_bytes =
		db::get_total_size_of_docker_repositories_for_workspace(
			connection,
			workspace_id,
		)
		.await? as usize;

	Ok(total_usage_so_far_in_bytes + additional_bytes >=
		free_limits::DOCKER_REPOSITORY_STORAGE_IN_BYTES)
}
