use axum::http::StatusCode;
use models::{api::workspace::container_registry::*, prelude::*};

use crate::prelude::*;

pub async fn delete_repository_image(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path:
					DeleteContainerRepositoryImagePath {
						workspace_id,
						repository_id,
						digest,
					},
				query: (),
				headers:
					DeleteContainerRepositoryImageRequestHeaders {
						user_agent: _,
						authorization: _,
					},
				body: DeleteContainerRepositoryImageRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, DeleteContainerRepositoryImageRequest>,
) -> Result<AppResponse<DeleteContainerRepositoryImageRequest>, ErrorType> {
	info!("Starting: Delete container repository image");

	// Get repository detail
	let repository_name = query!(
		r#"
		SELECT
			name
		FROM
			container_registry_repository
		WHERE
			id = $1 AND
			deleted IS NULL;
		"#,
		repository_id as _
	)
	.fetch_optional(&mut **database)
	.await?
	.map(|repo| repo.name)
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	let name = format!("{}/{}", workspace_id, repository_name);

	// Delete all tags for the given image
	query!(
		r#"
		DELETE FROM
			container_registry_repository_tag
		WHERE
			repository_id = $1 AND
			manifest_digest = $2;
		"#,
		repository_id as _,
		digest
	)
	.execute(&mut **database)
	.await?;

	// Delete container repository image with digest from database
	query!(
		r#"
		DELETE FROM
			container_registry_repository_manifest
		WHERE
			repository_id = $1 AND
			manifest_digest = $2;
		"#,
		repository_id as _,
		digest
	)
	.execute(&mut **database)
	.await?;

	super::delete_docker_repository_image_in_registry(&name, &user_data.username, &digest, &config)
		.await?;

	AppResponse::builder()
		.body(DeleteContainerRepositoryImageResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
