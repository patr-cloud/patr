use axum::http::StatusCode;
use models::{api::workspace::container_registry::*, prelude::*};

use crate::prelude::*;

pub async fn get_repository_info(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path:
					GetContainerRepositoryInfoPath {
						workspace_id: _,
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
		WITH RECURSIVE manifest_set AS (
			SELECT
				manifest_digest AS digest
			FROM
				container_registry_repository_manifest
			WHERE
				repository_id = $1
			UNION
			SELECT
				manifest_reference.referenced_digest AS digest
			FROM
				container_registry_manifest_reference manifest_reference
			INNER JOIN
				manifest_set
			ON
				manifest_set.digest = manifest_reference.manifest_digest
		),
		manifest_size AS (
			SELECT
				COALESCE(SUM(manifest.size), 0)::BIGINT AS size
			FROM
				container_registry_manifest manifest
			INNER JOIN
				manifest_set
			ON
				manifest_set.digest = manifest.digest
		),
		blob_set AS (
			SELECT
				image.config_blob_digest AS digest
			FROM
				container_registry_manifest_image image
			INNER JOIN
				manifest_set
			ON
				manifest_set.digest = image.manifest_digest
			UNION
			SELECT
				layer.blob_digest AS digest
			FROM
				container_registry_manifest_layer layer
			INNER JOIN
				manifest_set
			ON
				manifest_set.digest = layer.manifest_digest
		),
		blob_size AS (
			SELECT
				COALESCE(SUM(blob.size), 0)::BIGINT AS size
			FROM
				container_registry_blob blob
			INNER JOIN
				blob_set
			ON
				blob_set.digest = blob.digest
		)
		SELECT
			(manifest_size.size + blob_size.size)::BIGINT AS "size!"
		FROM
			manifest_size,
			blob_size;
		"#,
		repository_id as _,
	)
	.fetch_one(&mut **database)
	.await?
	.size as u64;

	let (last_updated, created) = query!(
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
			) AS "last_updated!",
			created
		FROM
			resource
		WHERE
			resource.id = $1;
		"#,
		repository_id as _
	)
	.fetch_one(&mut **database)
	.await
	.map(|row| (row.last_updated, row.created))?;

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
