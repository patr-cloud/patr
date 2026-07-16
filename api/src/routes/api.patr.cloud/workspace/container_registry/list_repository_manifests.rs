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
					ListResourceQueryProcessed {
						sort: _,
						search:
							ContainerRepositoryManifestInfoSearchParams {
								digest: digest_filter,
								size: size_filter,
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
				manifest.kind AS kind,
				manifest.artifact_type AS artifact_type,
				repository_manifest.created_at AS created,
				COALESCE(tag_agg.tags, ARRAY[]::TEXT[]) AS tags,
				COALESCE(platform_agg.platforms, '[]'::JSONB) AS platforms
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
				container_registry_manifest_image image
			ON
				image.manifest_digest = manifest.digest
			LEFT JOIN
				container_registry_blob config_blob
			ON
				config_blob.digest = image.config_blob_digest
			LEFT JOIN LATERAL (
				SELECT
					COALESCE(SUM(layer.size), 0)::BIGINT AS total_size
				FROM
					container_registry_manifest_layer layer
				WHERE
					layer.manifest_digest = repository_manifest.manifest_digest
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
			LEFT JOIN LATERAL (
				SELECT
					JSONB_AGG(
						JSONB_BUILD_OBJECT(
							'os', platform.os,
							'architecture', platform.architecture,
							'variant', platform.variant,
							'osVersion', platform.os_version
						)
					) AS platforms
				FROM (
					SELECT
						image.os,
						image.architecture,
						image.variant,
						image.os_version
					FROM
						container_registry_manifest_image image
					WHERE
						image.manifest_digest = manifest.digest
					UNION ALL
					SELECT
						reference.os,
						reference.architecture,
						reference.variant,
						reference.os_version
					FROM
						container_registry_manifest_reference reference
					WHERE
						reference.manifest_digest = manifest.digest AND
						reference.os IS NOT NULL AND
						reference.architecture IS NOT NULL
				) platform
			) platform_agg
			ON
				TRUE
			WHERE
				repository_manifest.repository_id = $1 AND
				repository.workspace_id = $2 AND
				repository.deleted IS NULL AND
				NOT EXISTS (
					SELECT
						1
					FROM
						container_registry_manifest_reference child_reference
					WHERE
						child_reference.referenced_digest =
							repository_manifest.manifest_digest
				)
		)
		SELECT
			manifests.digest AS "digest!",
			manifests.size AS "size!",
			manifests.kind AS "kind!: ManifestKind",
			manifests.artifact_type,
			manifests.created,
			manifests.tags AS "tags!: Vec<String>",
			manifests.platforms AS "platforms!: sqlx::types::Json<Vec<Platform>>",
			COUNT(*) OVER () AS "count!"
		FROM
			manifests
		WHERE
			($3::TEXT IS NULL OR manifests.digest ILIKE '%' || $3 || '%') AND
			($4::BIGINT IS NULL OR manifests.size >= $4) AND
			($5::BIGINT IS NULL OR manifests.size <= $5) AND
			($6::TIMESTAMPTZ IS NULL OR manifests.created >= $6) AND
			($7::TIMESTAMPTZ IS NULL OR manifests.created <= $7) AND
			(
				$8::TEXT[] IS NULL OR EXISTS (
					SELECT
						1
					FROM
						UNNEST(manifests.tags) AS tag_name
					WHERE
						tag_name = ANY($8::TEXT[])
				)
			)
		ORDER BY
			manifests.created DESC
		LIMIT $9
		OFFSET $10;
		"#,
		repository_id as _,
		workspace_id as _,
		digest_filter,
		size_filter.as_ref().map(|size| *size.start() as i64) as _,
		size_filter.as_ref().map(|size| *size.end() as i64) as _,
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
			kind: manifest.kind,
			platforms: manifest.platforms.0,
			artifact_type: manifest.artifact_type,
			created: manifest.created,
			tags: manifest.tags,
		}
	})
	.collect();

	if page != 0 && total_count == 0 {
		return Err(ErrorType::PageOutOfBounds);
	}

	AppResponse::builder()
		.body(ListContainerRepositoryManifestsResponse { manifests })
		.headers(ListContainerRepositoryManifestsResponseHeaders {
			total_count: TotalCountHeader(total_count as _),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
