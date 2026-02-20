use axum::http::StatusCode;
use models::{api::workspace::container_registry::*, prelude::*};

use crate::prelude::*;

pub async fn delete_repository_manifest(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path:
					DeleteContainerRepositoryManifestPath {
						workspace_id: _,
						repository_id,
						digest,
					},
				query: (),
				headers:
					DeleteContainerRepositoryManifestRequestHeaders {
						user_agent: _,
						authorization: _,
					},
				body: DeleteContainerRepositoryManifestRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		user_data: _,
		state: _,
	}: AuthenticatedAppRequest<'_, DeleteContainerRepositoryManifestRequest>,
) -> Result<AppResponse<DeleteContainerRepositoryManifestRequest>, ErrorType> {
	info!("Starting: Delete container repository manifest");

	// Delete all tags for the given manifest
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

	// Delete container repository manifest with digest from database
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
		.body(DeleteContainerRepositoryManifestResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
