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

	db::delete_docker_repository_image(connection, repository_id, digest)
		.await?;

	let response_code = reqwest::Client::new()
		.delete(format!(
			"{}://{}/v2/{}/manifests/{}",
			if config.docker_registry.registry_url.starts_with("localhost") {
				"http"
			} else {
				"https"
			},
			config.docker_registry.registry_url,
			repository.name,
			digest
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

	let images = db::get_list_of_digests_for_docker_repository(
		connection,
		repository_id,
	)
	.await?;

	db::delete_all_tags_for_docker_repository(connection, repository_id)
		.await?;
	db::delete_all_images_for_docker_repository(connection, repository_id)
		.await?;

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
