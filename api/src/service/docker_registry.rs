use eve_rs::AsError;

use crate::{
	db,
	error,
	models::{rbac, RegistryToken, RegistryTokenAccess},
	utils::{get_current_time, settings::Settings, Error},
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

	let repo_name = format!(
		"{}/{}",
		db::get_workspace_info(connection, &repository.workspace_id)
			.await?
			.status(500)?
			.name,
		repository.name
	);

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

	let god_user = db::get_user_by_user_id(
		connection,
		rbac::GOD_USER_ID.get().unwrap().as_bytes(),
	)
	.await?
	.unwrap();

	let iat = get_current_time().as_secs();

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
				&config,
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

	let god_user = db::get_user_by_user_id(
		connection,
		rbac::GOD_USER_ID.get().unwrap().as_bytes(),
	)
	.await?
	.unwrap();

	let iat = get_current_time().as_secs();

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
					&config,
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

	Ok(())
}
