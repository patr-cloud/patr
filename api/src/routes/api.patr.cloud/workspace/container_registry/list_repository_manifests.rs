use axum::http::StatusCode;
use models::{api::workspace::container_registry::*, prelude::*};

use crate::prelude::*;

pub async fn list_repository_manifests(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path:
					ListContainerRepositoryManifestsPath {
						workspace_id,
						repository_id,
					},
				query:
					ListResourceQuery {
						sort: _,
						search:
							ContainerRepositoryManifestInfoSearchParams {
								digest: digest_filter,
								size: size_filter,
								platform: platform_filter,
								created: created_filter,
								tags: tags_filter,
							},
						count,
						page,
						additional_query: (),
					},
				headers:
					ListContainerRepositoryManifestsRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: ListContainerRepositoryManifestsRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		user_data: _,
		state: _,
	}: AuthenticatedAppRequest<'_, ListContainerRepositoryManifestsRequest>,
) -> Result<AppResponse<ListContainerRepositoryManifestsRequest>, ErrorType> {
	info!("Listing container registry repository manifests");

	let mut total_count = 0;

	let manifests = query!(
		r#"
		WITH manifests AS (
			SELECT
				repository_manifest.manifest_digest AS digest,
				(
					manifest.size +
					COALESCE(config_blob.size, 0) +
					COALESCE(layer_size.total_size, 0)
				)::BIGINT AS size,
				manifest.platform,
				repository_manifest.created_at AS created,
				COALESCE(tag_agg.tags, ARRAY[]::TEXT[]) AS tags
			FROM
				container_registry_repository_manifest repository_manifest
			INNER JOIN
				container_registry_repository repository
			ON
				repository.id = repository_manifest.repository_id
			INNER JOIN
				container_registry_manifest manifest
			ON
				manifest.digest = repository_manifest.manifest_digest
			LEFT JOIN
				container_registry_blob config_blob
			ON
				config_blob.digest = manifest.config_blob_digest
			LEFT JOIN LATERAL (
				SELECT
					COALESCE(SUM(layer_blob.size), 0)::BIGINT AS total_size
				FROM
					container_registry_manifest_blob manifest_blob
				INNER JOIN
					container_registry_blob layer_blob
				ON
					layer_blob.digest = manifest_blob.blob_digest
				WHERE
					manifest_blob.manifest_digest = repository_manifest.manifest_digest
			) layer_size
			ON
				TRUE
			LEFT JOIN LATERAL (
				SELECT
					ARRAY_AGG(tag.name ORDER BY tag.last_updated DESC) AS tags
				FROM
					container_registry_repository_tag tag
				WHERE
					tag.repository_id = repository_manifest.repository_id AND
					tag.manifest_digest = repository_manifest.manifest_digest
			) tag_agg
			ON
				TRUE
			WHERE
				repository_manifest.repository_id = $1 AND
				repository.workspace_id = $2 AND
				repository.deleted IS NULL
		)
		SELECT
			manifests.digest AS "digest!",
			manifests.size AS "size!",
			manifests.platform AS "platform!",
			manifests.created,
			manifests.tags AS "tags!: Vec<String>",
			COUNT(*) OVER () AS "count!"
		FROM
			manifests
		WHERE
			($3::TEXT IS NULL OR manifests.digest ILIKE '%' || $3 || '%') AND
			($4::BIGINT IS NULL OR manifests.size >= $4) AND
			($5::BIGINT IS NULL OR manifests.size <= $5) AND
			($6::TEXT IS NULL OR manifests.platform ILIKE '%' || $6 || '%') AND
			($7::TIMESTAMPTZ IS NULL OR manifests.created >= $7) AND
			($8::TIMESTAMPTZ IS NULL OR manifests.created <= $8) AND
			(
				$9::TEXT[] IS NULL OR EXISTS (
					SELECT
						1
					FROM
						UNNEST(manifests.tags) AS tag_name
					WHERE
						tag_name = ANY($9::TEXT[])
				)
			)
		ORDER BY
			manifests.created DESC
		LIMIT $10
		OFFSET $11;
		"#,
		repository_id as _,
		workspace_id as _,
		digest_filter,
		size_filter.as_ref().map(|size| *size.start() as i64) as _,
		size_filter.as_ref().map(|size| *size.end() as i64) as _,
		platform_filter,
		created_filter.as_ref().map(|range| range.start()) as _,
		created_filter.as_ref().map(|range| range.end()) as _,
		tags_filter.as_deref(),
		count as i32,
		(page * count) as i32
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|manifest| {
		total_count = manifest.count;
		ContainerRepositoryManifestInfo {
			digest: manifest.digest,
			size: manifest.size as u64,
			platform: manifest.platform,
			created: manifest.created,
			tags: manifest.tags,
		}
	})
	.collect();

	AppResponse::builder()
		.body(ListContainerRepositoryManifestsResponse { manifests })
		.headers(ListContainerRepositoryManifestsResponseHeaders {
			total_count: TotalCountHeader(total_count as _),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
