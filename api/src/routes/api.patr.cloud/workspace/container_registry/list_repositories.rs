use axum::http::StatusCode;
use models::{api::workspace::container_registry::*, prelude::*};

use crate::prelude::*;

pub async fn list_repositories(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ListContainerRepositoriesPath { workspace_id },
				query:
					ListResourceQueryProcessed {
						sort: sort_order,
						search:
							ContainerRepositorySearchParams {
								name: name_filter,
								size: size_filter,
								last_updated: last_updated_filter,
								created: created_filter,
							},
						count,
						page,
						additional_query: (),
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
		user_data,
		state: _,
	}: AuthenticatedAppRequest<'_, ListContainerRepositoriesRequest>,
) -> Result<AppResponse<ListContainerRepositoriesRequest>, ErrorType> {
	info!("Listing container registry repositories");

	let mut total_count = 0;

	let repositories = query!(
		r#"
		WITH repos AS (
			SELECT
				container_registry_repository.id,
				container_registry_repository.name,
				COALESCE(
					(
						WITH RECURSIVE manifest_set AS (
							SELECT
								manifest_digest AS digest
							FROM
								container_registry_repository_manifest
							WHERE
								repository_id = container_registry_repository.id
							UNION
							SELECT
								manifest_reference.referenced_digest AS digest
							FROM
								container_registry_manifest_reference manifest_reference
							INNER JOIN
								manifest_set
							ON
								manifest_set.digest = manifest_reference.digest
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
								manifest.config_blob_digest AS digest
							FROM
								container_registry_manifest manifest
							INNER JOIN
								manifest_set
							ON
								manifest_set.digest = manifest.digest
							WHERE
								manifest.config_blob_digest IS NOT NULL
							UNION
							SELECT
								manifest_blob.blob_digest AS digest
							FROM
								container_registry_manifest_blob manifest_blob
							INNER JOIN
								manifest_set
							ON
								manifest_set.digest = manifest_blob.manifest_digest
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
							(manifest_size.size + blob_size.size)::BIGINT
						FROM
							manifest_size,
							blob_size
					),
					0
				)::BIGINT AS size,
				GREATEST(
					resource.created,
					(
						SELECT
							MAX(last_updated)
						FROM
							container_registry_repository_tag
						WHERE
							repository_id = container_registry_repository.id
					),
					(
						SELECT
							MAX(created_at)
						FROM
							container_registry_repository_manifest
						WHERE
							repository_id = container_registry_repository.id
					)
				) AS "last_updated",
				resource.created,
				COUNT(*) OVER () AS "count"
			FROM
				container_registry_repository
			INNER JOIN
				resource
			ON
				resource.id = container_registry_repository.id
			WHERE
				container_registry_repository.workspace_id = $1 AND
				container_registry_repository.deleted IS NULL
		)
		SELECT
			repos.id,
			repos.name,
			repos.size AS "size!",
			repos.last_updated AS "last_updated!",
			repos.created,
			repos.count AS "count!"
		FROM
			repos
		INNER JOIN
			RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID($2, $3) AS permission_resource
		ON
			repos.id = permission_resource.id
		WHERE
			($4::TEXT IS NULL OR repos.name ILIKE '%' || $4 || '%') AND
			($5::BIGINT IS NULL OR repos.size >= $5) AND
			($6::BIGINT IS NULL OR repos.size <= $6) AND
			($7::TIMESTAMPTZ IS NULL OR repos.last_updated >= $7) AND
			($8::TIMESTAMPTZ IS NULL OR repos.last_updated <= $8) AND
			($9::TIMESTAMPTZ IS NULL OR repos.created >= $9) AND
			($10::TIMESTAMPTZ IS NULL OR repos.created <= $10)
		ORDER BY
			repos.created
		LIMIT $11
		OFFSET $12;
		"#,
		workspace_id as _,
		user_data.login_id as _,
		Permission::ContainerRegistryRepository(ContainerRegistryRepositoryPermission::View)
			.to_string(),
		name_filter,
		size_filter.as_ref().map(|size| *size.start() as i64) as _,
		size_filter.as_ref().map(|size| *size.end() as i64) as _,
		last_updated_filter.as_ref().map(|range| range.start()) as _,
		last_updated_filter.as_ref().map(|range| range.end()) as _,
		created_filter.as_ref().map(|range| range.start()) as _,
		created_filter.as_ref().map(|range| range.end()) as _,
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
