use api_models::models::workspace::docker_registry::{
	DockerRepositoryImageInfo,
	DockerRepositoryTagInfo,
};
use eve_rs::AsError;

use crate::{
	db,
	error,
	utils::{settings::Settings, Error},
	Database,
};

pub async fn delete_docker_repository_image(
	connection: &mut <Database as sqlx::Database>::Connection,
	repository_id: &[u8],
	digest: &str,
	config: &Settings,
) -> Result<(), Error> {
	let repository = db::get_docker_repository_by_id(connection, repository_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	// First, delete all tags for the given image
	let tags = vec![DockerRepositoryTagInfo {
		tag: todo!(),
		last_updated: todo!(),
	}];
	for tag in tags {
		// TODO: delete all tags
	}

	// TODO: delete all images

	// let response_code = reqwest::Client::new()
	// 	.delete(format!(
	// 		"{}://{}/v2/{}/manifests/{}",
	// 		if config.docker_registry.registry_url.starts_with("localhost") {
	// 			"http"
	// 		} else {
	// 			"https"
	// 		},
	// 		config.docker_registry.registry_url,
	// 		repository.name,
	// 		digest
	// 	))
	// 	.header(
	// 		reqwest::header::ACCEPT,
	// 		"application/vnd.docker.distribution.events.v1+json",
	// 	)
	// 	.send()
	// 	.await?
	// 	.status();

	// if response_code == 404 {
	// 	return Err(Error::empty()
	// 		.status(404)
	// 		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string()));
	// } else if !response_code.is_success() {
	// 	return Err(Error::empty());
	// }

	Ok(())
}

pub async fn delete_docker_repository(
	connection: &mut <Database as sqlx::Database>::Connection,
	repository_id: &[u8],
	config: &Settings,
) -> Result<(), Error> {
	let repository = db::get_docker_repository_by_id(connection, repository_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let images = vec![DockerRepositoryImageInfo {
		digest: todo!(),
		size: todo!(),
		created: todo!(),
	}];

	// TODO: delete all images and tagsd for docker repository

	db::update_docker_repository_name(
		connection,
		repository_id,
		&format!(
			"patr-deleted: {}-{}",
			repository.name,
			hex::encode(&repository_id)
		),
	)
	.await?;

	let client = reqwest::Client::new();

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
				repository.name,
				image.digest
			))
			.header(
				reqwest::header::ACCEPT,
				"application/vnd.docker.distribution.events.v1+json",
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

	Ok(())
}
