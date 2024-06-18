use axum::http::StatusCode;
use models::{api::workspace::container_registry::*, prelude::*};

use crate::prelude::*;

pub async fn list_repositories(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ListContainerRepositoriesPath { workspace_id },
				query: Paginated {
					data: (),
					count,
					page,
				},
				headers:
					ListContainerRepositoriesRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: ListContainerRepositoriesRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		config: _,
		user_data,
	}: AuthenticatedAppRequest<'_, ListContainerRepositoriesRequest>,
) -> Result<AppResponse<ListContainerRepositoriesRequest>, ErrorType> {
	info!("Listing container registry repositories");

	let mut total_count = 0;

	let repositories = query!(
		r#" 
		SELECT
			container_registry_repository.id,
			container_registry_repository.name,
			COALESCE(
				(
					SELECT
						SUM(container_registry_repository_blob.size)
					FROM
						container_registry_repository_manifest
					LEFT JOIN
						container_registry_manifest_blob
					ON
						container_registry_repository_manifest.manifest_digest = container_registry_manifest_blob.manifest_digest
					LEFT JOIN
						container_registry_repository_blob
					ON
						container_registry_manifest_blob.blob_digest = container_registry_repository_blob.blob_digest
					WHERE
						container_registry_repository_manifest.repository_id = container_registry_repository.id
				),
				0
			)::BIGINT AS "size!",
			GREATEST(
				(
					SELECT
						MAX(container_registry_repository_tag.last_updated)
					FROM
						container_registry_repository_tag
					WHERE
						repository_id = container_registry_repository.id
				),
				(
					SELECT
						MAX(container_registry_repository_manifest.created)
					FROM
						container_registry_repository_manifest
					WHERE
						repository_id = container_registry_repository.id
				),
				resource.created
			) AS "last_updated!",
			resource.created,
			COUNT(*) OVER () AS "count!"
		FROM
			container_registry_repository
		INNER JOIN
			resource
		ON
			resource.id = container_registry_repository.id
		INNER JOIN
			RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID($2, $3) AS permission_resource
		ON
			container_registry_repository.id = permission_resource.id
		WHERE
			container_registry_repository.workspace_id = $1 AND
			container_registry_repository.deleted IS NULL
		ORDER BY
			resource.created
		LIMIT $4
		OFFSET $5;
		"#,
		workspace_id as _,
		user_data.login_id as _,
		"TODO permission_name",
		count as i32,
		(page * count) as i32
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|repo| {
		total_count = repo.count;
		WithId::new(
			repo.id,
			ContainerRepository {
				name: repo.name,
				size: repo.size as u64,
				last_updated: repo.last_updated,
				created: repo.created,
			},
		)
	})
	.collect();

	AppResponse::builder()
		.body(ListContainerRepositoriesResponse { repositories })
		.headers(ListContainerRepositoriesResponseHeaders {
			total_count: TotalCountHeader(total_count as _),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
