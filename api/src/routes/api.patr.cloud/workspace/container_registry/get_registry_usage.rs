use axum::http::StatusCode;
use models::{api::workspace::container_registry::*, prelude::*};

use crate::prelude::*;

pub async fn get_registry_usage(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: GetContainerRegistryUsagePath { workspace_id },
				query: (),
				headers:
					GetContainerRegistryUsageRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: GetContainerRegistryUsageRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		user_data: _,
		state: _,
	}: AuthenticatedAppRequest<'_, GetContainerRegistryUsageRequest>,
) -> Result<AppResponse<GetContainerRegistryUsageRequest>, ErrorType> {
	info!("Getting container registry usage for workspace");

	// Storage is deduplicated: a blob shared by several images is counted once.
	// `workspace_manifests` gathers every manifest reachable from the
	// workspace's live repositories; `used_bytes` sums the distinct config +
	// layer blobs those manifests point at.
	let usage = query!(
		r#"
		WITH workspace_manifests AS (
			SELECT DISTINCT
				repository_manifest.manifest_digest AS digest
			FROM
				container_registry_repository_manifest repository_manifest
			INNER JOIN
				container_registry_repository repository
			ON
				repository.id = repository_manifest.repository_id
			WHERE
				repository.workspace_id = $1 AND
				repository.deleted IS NULL
		)
		SELECT
			COALESCE(
				(
					SELECT
						SUM(blob.size)
					FROM
						container_registry_blob blob
					WHERE
						blob.digest IN (
							SELECT
								image.config_blob_digest
							FROM
								container_registry_manifest_image image
							WHERE
								image.manifest_digest IN (
									SELECT digest FROM workspace_manifests
								)
							UNION
							SELECT
								layer.blob_digest
							FROM
								container_registry_manifest_layer layer
							WHERE
								layer.manifest_digest IN (
									SELECT digest FROM workspace_manifests
								)
						)
				),
				0
			)::BIGINT AS "used_bytes!",
			(
				SELECT
					COUNT(*)
				FROM
					container_registry_repository repository
				WHERE
					repository.workspace_id = $1 AND
					repository.deleted IS NULL
			)::BIGINT AS "repository_count!",
			(
				SELECT
					COUNT(*)
				FROM
					container_registry_manifest manifest
				WHERE
					manifest.kind = 'image' AND
					manifest.digest IN (
						SELECT digest FROM workspace_manifests
					)
			)::BIGINT AS "image_count!";
		"#,
		workspace_id as _,
	)
	.fetch_one(&mut **database)
	.await?;

	AppResponse::builder()
		.body(GetContainerRegistryUsageResponse {
			used_bytes: usage.used_bytes as u64,
			repository_count: usage.repository_count as u64,
			image_count: usage.image_count as u64,
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
