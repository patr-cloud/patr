use axum::http::StatusCode;
use models::{api::workspace::container_registry::*, prelude::*};

use crate::prelude::*;

pub async fn get_repository_image_details(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path:
					GetContainerRepositoryImageDetailsPath {
						workspace_id: _,
						repository_id,
						digest_or_tag,
					},
				query: (),
				headers:
					GetContainerRepositoryImageDetailsRequestHeaders {
						user_agent: _,
						authorization: _,
					},
				body: GetContainerRepositoryImageDetailsRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		config: _,
		user_data: _,
	}: AuthenticatedAppRequest<'_, GetContainerRepositoryImageDetailsRequest>,
) -> Result<AppResponse<GetContainerRepositoryImageDetailsRequest>, ErrorType> {
	info!("Starting: Get image details");

	let (image_digest, image_created) = query!(
		r#"
		SELECT
			manifest_digest,
			created
		FROM
			container_registry_repository_manifest
		WHERE
			repository_id = $1 AND
			manifest_digest = $2;
		"#,
		repository_id as _,
		digest_or_tag
	)
	.fetch_optional(&mut **database)
	.await?
	.map(|image| (image.manifest_digest, image.created))
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	let image_tags = query!(
		r#"
		SELECT
			tag,
			last_updated
		FROM
			container_registry_repository_tag
		WHERE
			repository_id = $1 AND
			manifest_digest = $2;
		"#,
		repository_id as _,
		digest_or_tag
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|row| row.tag)
	.collect();

	let image_size = query!(
		r#"
		SELECT
			COALESCE(SUM(container_registry_repository_blob.size), 0)::BIGINT AS "image_size"
		FROM
			container_registry_manifest_blob
		INNER JOIN
			container_registry_repository_blob
		ON
			container_registry_manifest_blob.blob_digest
			= container_registry_repository_blob.blob_digest
		INNER JOIN
			container_registry_repository_manifest
		ON
			container_registry_manifest_blob.manifest_digest
			= container_registry_repository_manifest.manifest_digest
		WHERE
			container_registry_repository_manifest.repository_id = $1 AND
			container_registry_repository_manifest.manifest_digest = $2;
		"#,
		repository_id as _,
		digest_or_tag
	)
	.fetch_one(&mut **database)
	.await
	.map(|repo| repo.image_size)?
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	AppResponse::builder()
		.body(GetContainerRepositoryImageDetailsResponse {
			digest: image_digest,
			size: image_size as u64,
			created: image_created,
			tags: image_tags,
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
