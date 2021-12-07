use std::ops::DerefMut;

use api_models::models::workspace::docker_registry::{
	DockerRepository,
	DockerRepositoryTagInfo,
	GetDockerRepositoryInfoResponse,
};
use eve_rs::AsError;
use uuid::Uuid;

use crate::{
	db,
	error,
	models::{
		db_mapping::DockerRepository as DbDockerRepository,
		rbac,
		DockerRegistryImageListTagsResponse,
		RegistryToken,
		RegistryTokenAccess,
	},
	service,
	utils::{get_current_time, settings::Settings, Error},
	Database,
};

pub async fn get_docker_repository_info(
	connection: &mut <Database as sqlx::Database>::Connection,
	repository_id: &[u8],
) -> Result<GetDockerRepositoryInfoResponse, Error> {
	let DbDockerRepository {
		id,
		name,
		workspace_id: _,
	} = db::get_docker_repository_by_id(connection, repository_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let mut size = 0;
	let mut last_updated = 0;

	let images = db::get_list_of_digests_for_docker_repository(
		connection,
		&repository_id,
	)
	.await?;
	images.iter().for_each(|image| {
		size += image.size;
		last_updated = last_updated.max(image.created);
	});

	Ok(GetDockerRepositoryInfoResponse {
		repository: DockerRepository {
			id: Uuid::from_slice(&id)?,
			name,
			size,
		},
		images,
		last_updated,
	})
}

#[allow(dead_code)]
pub async fn get_docker_repository_tags(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &[u8],
	repository: &str,
	config: &Settings,
) -> Result<Vec<DockerRepositoryTagInfo>, Error> {
	let app = service::get_app().clone();
	let god_username = db::get_user_by_user_id(
		app.database.acquire().await?.deref_mut(),
		rbac::GOD_USER_ID.get().unwrap().as_bytes(),
	)
	.await?
	.status(500)?
	.username;

	let workspace_name = db::get_workspace_info(&mut *connection, workspace_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?
		.name
		.to_lowercase();

	let iat = get_current_time().as_secs();
	let token = RegistryToken::new(
		config.docker_registry.issuer.clone(),
		iat,
		god_username.clone(),
		config,
		vec![RegistryTokenAccess {
			name: format!("{}/{}", workspace_name, repository),
			actions: vec!["pull".to_string()],
			r#type: "repository".to_string(),
		}],
	)
	.to_string(
		config.docker_registry.private_key.as_ref(),
		config.docker_registry.public_key_der.as_ref(),
	)?;

	let tags = reqwest::Client::new()
		.get(format!(
			"{}://{}/v2/{}/{}/tags/list",
			if cfg!(debug_assertions) {
				"http"
			} else {
				"https"
			},
			config.docker_registry.registry_url,
			workspace_name,
			repository
		))
		.basic_auth(god_username, Some(token))
		.send()
		.await?
		.json::<DockerRegistryImageListTagsResponse>()
		.await?
		.tags
		.into_iter()
		.map(|tag| DockerRepositoryTagInfo {
			tag,
			last_updated: 0,
		})
		.collect();

	Ok(tags)
}
