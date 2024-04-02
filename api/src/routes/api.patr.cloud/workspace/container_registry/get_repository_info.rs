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
		config: _,
		user_data: _,
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
			COALESCE(SUM(size), 0)::BIGINT as "size!"
		FROM
			container_registry_repository
		INNER JOIN
			container_registry_repository_manifest
		ON
			container_registry_repository.id 
			= container_registry_repository_manifest.repository_id
		INNER JOIN
			container_registry_manifest_blob
		ON
			container_registry_repository_manifest.manifest_digest 
			= container_registry_manifest_blob.manifest_digest
		INNER JOIN
			container_registry_repository_blob
		ON
			container_registry_manifest_blob.blob_digest 
			= container_registry_repository_blob.blob_digest
		WHERE
			container_registry_repository.workspace_id = $1;
		"#,
		workspace_id as _,
	)
	.fetch_one(&mut **database)
	.await
	.map(|repo| repo.size as u64)?;

	let last_updated = query!(
		r#"
		SELECT 
			GREATEST(
				resource.created, 
				(
					SELECT 
						COALESCE(created, TO_TIMESTAMP(0)) 
					FROM 
						container_registry_repository_manifest 
					WHERE 
						repository_id = $1
					ORDER BY
						created DESC
					LIMIT 1
				), 
				(
					SELECT 
						COALESCE(last_updated, TO_TIMESTAMP(0)) 
					FROM 
						container_registry_repository_tag 
					WHERE 
						repository_id = $1
					ORDER BY
						created DESC
					LIMIT 1
				)
			) as "last_updated!"
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
			MIN(created) AS created
		FROM
			container_registry_repository_manifest
		WHERE
			repository_id = $1;
		"#,
		repository_id as _
	)
	.fetch_one(&mut **database)
	.await
	.map(|repo| repo.created)?
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
