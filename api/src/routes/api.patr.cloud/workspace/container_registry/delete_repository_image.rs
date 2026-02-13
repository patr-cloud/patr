use axum::http::StatusCode;
use models::{api::workspace::container_registry::*, prelude::*};

use crate::prelude::*;

pub async fn delete_repository_image(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path:
					DeleteContainerRepositoryImagePath {
						workspace_id: _,
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
		user_data: _,
		state: _,
	}: AuthenticatedAppRequest<'_, DeleteContainerRepositoryImageRequest>,
) -> Result<AppResponse<DeleteContainerRepositoryImageRequest>, ErrorType> {
	info!("Starting: Delete container repository image");

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

	AppResponse::builder()
		.body(DeleteContainerRepositoryImageResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
