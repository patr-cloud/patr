use api_models::models::workspace::docker_registry::{
	DockerRepository,
	DockerRepositoryImageInfo,
	DockerRepositoryTagInfo,
	GetDockerRepositoryInfoResponse,
};
use eve_rs::AsError;
use uuid::Uuid;

use crate::{
	db,
	error,
	models::db_mapping::DockerRepository as DbDockerRepository,
	utils::{settings::Settings, Error},
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

	let size =
		db::get_total_size_of_docker_repository(connection, &repository_id)
			.await?;
	let mut last_updated = 0;

	let images = db::get_list_of_digests_for_docker_repository(
		connection,
		&repository_id,
	)
	.await?;
	images.iter().for_each(|image| {
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

pub async fn get_docker_repository_image_details(
	connection: &mut <Database as sqlx::Database>::Connection,
	repository_id: &[u8],
	digest: &str,
) -> Result<Vec<DockerRepositoryTagInfo>, Error> {
	let tags = db::get_tags_for_docker_repository_image(
		connection,
		repository_id,
		digest,
	)
	.await?;

	Ok(tags)
}
