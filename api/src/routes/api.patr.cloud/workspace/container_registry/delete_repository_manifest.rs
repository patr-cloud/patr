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
						digest_or_tag,
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

	// Refuse if any live deployment references a tag that points at this
	// manifest (handles both digest and tag name input).
	let in_use = query!(
		r#"
		SELECT
			1 AS "x!"
		FROM
			deployment
		WHERE
			deployment.repository_id = $1 AND
			deployment.deleted IS NULL AND
			deployment.image_tag IN (
				SELECT
					name
				FROM
					container_registry_repository_tag
				WHERE
					repository_id = $1 AND
					(manifest_digest = $2 OR name = $2)
			)
		LIMIT 1;
		"#,
		repository_id as _,
		digest_or_tag,
	)
	.fetch_optional(&mut **database)
	.await?
	.is_some();

	if in_use {
		return Err(ErrorType::ResourceInUse);
	}

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
		digest_or_tag
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
			manifest_digest = $2
		RETURNING manifest_digest;
		"#,
		repository_id as _,
		digest_or_tag
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	AppResponse::builder()
		.body(DeleteContainerRepositoryManifestResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
