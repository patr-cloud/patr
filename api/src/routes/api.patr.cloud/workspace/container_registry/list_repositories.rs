use axum::http::StatusCode;
use models::{api::workspace::container_registry::*, prelude::*};

use crate::prelude::*;

pub async fn list_repositories(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ListContainerRepositoriesPath { workspace_id },
				query: _,
				headers,
				body,
			},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, ListContainerRepositoriesRequest>,
) -> Result<AppResponse<ListContainerRepositoriesRequest>, ErrorType> {
	info!("Starting: List container repositories");

	let repositories = query!(
		r#" 
		SELECT
			container_registry_repository.id AS id,
			container_registry_repository.name AS name,
			COALESCE(SUM(container_registry_repository_blob.size), 0)::BIGINT AS size,
			MAX(container_registry_repository_tag.last_updated) AS last_updated,
			MIN(container_registry_repository_manifest.created) AS created
		FROM
			container_registry_repository
		LEFT JOIN
			container_registry_repository_manifest
		ON
			container_registry_repository.id 
			= container_registry_repository_manifest.repository_id
		LEFT JOIN
			container_registry_repository_tag
		ON
			container_registry_repository.id 
			= container_registry_repository_tag.repository_id
		LEFT JOIN
			container_registry_manifest_blob
		ON
			container_registry_repository_manifest.manifest_digest 
			= container_registry_manifest_blob.manifest_digest
		LEFT JOIN
			container_registry_repository_blob
		ON
			container_registry_manifest_blob.blob_digest 
			= container_registry_repository_blob.blob_digest
		WHERE
			container_registry_repository.workspace_id = $1
		GROUP BY
			container_registry_repository.id, container_registry_repository.name;
		"#,
		workspace_id as _
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|repo| {
		WithId::new(
			repo.id,
			ContainerRepository {
				name: repo.name,
				size: repo.size.unwrap() as u64,
				last_updated: repo.last_updated.unwrap(),
				created: repo.created.unwrap(),
			},
		)
	})
	.collect();

	let total_count = query!(
		r#" 
		SELECT
			COUNT(*) AS count
		FROM
			container_registry_repository
		WHERE
			workspace_id = $1;
		"#,
		workspace_id as _
	)
	.fetch_one(&mut **database)
	.await
	.map(|repo| repo.count)?
	.ok_or(ErrorType::server_error(
		"Failed to get total repository count",
	))?;

	AppResponse::builder()
		.body(ListContainerRepositoriesResponse { repositories })
		.headers(ListContainerRepositoriesResponseHeaders {
			total_count: TotalCountHeader(total_count as usize),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
