use axum::http::StatusCode;
use models::{api::workspace::container_registry::*, prelude::*};

use crate::prelude::*;

pub async fn get_repository_info(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: GetContainerRepositoryInfoPath {
					workspace_id,
					repository_id,
				},
				query: (),
				headers:
					GetContainerRepositoryInfoRequestHeaders {
						user_agent: _,
						authorization: _,
					},
				body: GetContainerRepositoryInfoRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		user_data: _,
		state: _,
	}: AuthenticatedAppRequest<'_, GetContainerRepositoryInfoRequest>,
) -> Result<AppResponse<GetContainerRepositoryInfoRequest>, ErrorType> {
	info!("Starting: Get repository info");

	// Check if repository exist and get info
	let name = query!(
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

	let size = query!(
		r#"
		SELECT
			COALESCE(SUM(container_registry_blob.size), 0)::BIGINT AS "size!"
		FROM
			container_registry_blob
		INNER JOIN
			container_registry_repository_manifest
		ON
			container_registry_blob.digest = container_registry_repository_manifest.manifest_digest
		WHERE
			container_registry_repository_manifest.repository_id = $1;
		"#,
		repository_id as _,
	)
	.fetch_one(&mut **database)
	.await?
	.size as u64;

	let last_updated = query!(
		r#"
		SELECT
			GREATEST(
				resource.created,
				(
					SELECT
						MAX(last_updated)
					FROM
						container_registry_repository_tag
					WHERE
						repository_id = $1
				),
				(
					SELECT
						MAX(created_at)
					FROM
						container_registry_repository_manifest
					WHERE
						repository_id = $1
				)
			) AS "last_updated!"
		FROM
			resource
		WHERE
			resource.id = $1;
		"#,
		repository_id as _
	)
	.fetch_one(&mut **database)
	.await
	.map(|row| row.last_updated)?;

	let created = query!(
		r#"
		SELECT
			MIN(created_at) AS created_at
		FROM
			container_registry_repository_manifest
		WHERE
			repository_id = $1;
		"#,
		repository_id as _
	)
	.fetch_one(&mut **database)
	.await
	.map(|repo| repo.created_at)?
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	AppResponse::builder()
		.body(GetContainerRepositoryInfoResponse {
			repository: ContainerRepository {
				name,
				size,
				last_updated,
				created,
			},
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
