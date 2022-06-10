use api_models::utils::Uuid;
use eve_rs::AsError;

use crate::{
	db,
	error,
	models::{rbac, RegistryToken, RegistryTokenAccess, RepoStorage},
	utils::{get_current_time, settings::Settings, Error},
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

	let repo_name = format!(
		"{}/{}",
		db::get_workspace_info(connection, &repository.workspace_id)
			.await?
			.status(500)?
			.name,
		repository.name
	);

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

	let god_user =
		db::get_user_by_user_id(connection, rbac::GOD_USER_ID.get().unwrap())
			.await?
			.unwrap();

	let iat = get_current_time().as_secs();

	log::trace!("request_id: {} - Deleting docker repository image with digest: {} from the registry", request_id, digest);
	let response_code = reqwest::Client::new()
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
				iat,
				god_user.username.clone(),
				config,
				vec![RegistryTokenAccess {
					r#type: "repository".to_string(),
					name: repo_name,
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
		.await?
		.status();

	if response_code == 404 {
		return Err(Error::empty()
			.status(404)
			.body(error!(RESOURCE_DOES_NOT_EXIST).to_string()));
	} else if !response_code.is_success() {
		return Err(Error::empty());
	}
	log::trace!("request_id: {} - Deleting docker repository image with digest: {} from the registry was successful", request_id, digest);

	Ok(())
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

	let repo_name = format!(
		"{}/{}",
		db::get_workspace_info(connection, &repository.workspace_id)
			.await?
			.status(500)?
			.name,
		repository.name
	);

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
	db::update_docker_repository_name(
		connection,
		repository_id,
		&format!(
			"patr-deleted: {}-{}",
			repository.name,
			repository_id.as_str()
		),
	)
	.await?;

	let client = reqwest::Client::new();

	let god_user =
		db::get_user_by_user_id(connection, rbac::GOD_USER_ID.get().unwrap())
			.await?
			.unwrap();

	let iat = get_current_time().as_secs();

	log::trace!("request_id: {} - Deleting docker images of the repositories from the registry", request_id);
	for image in images {
		let response_code = client
			.delete(format!(
				"{}://{}/v2/{}/manifests/{}",
				if config.docker_registry.registry_url.starts_with("localhost")
				{
					"http"
				} else {
					"https"
				},
				config.docker_registry.registry_url,
				repo_name,
				image.digest
			))
			.bearer_auth(
				RegistryToken::new(
					config.docker_registry.issuer.clone(),
					iat,
					god_user.username.clone(),
					config,
					vec![RegistryTokenAccess {
						r#type: "repository".to_string(),
						name: repo_name.clone(),
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
			.await?
			.status();

		if response_code == 404 {
			return Err(Error::empty()
				.status(404)
				.body(error!(RESOURCE_DOES_NOT_EXIST).to_string()));
		} else if !response_code.is_success() {
			return Err(Error::empty());
		}
	}

	log::trace!("request_id: {} - Deleting docker repository from the registry was successful", request_id);
	Ok(())
}

pub async fn get_storage_limit_for_repository(
	_repository_id: &Uuid,
) -> RepoStorage {
	// for now use 10 GB as limit for all repo
	RepoStorage::Limited(10 * 1024 * 1024 * 1024)
}
