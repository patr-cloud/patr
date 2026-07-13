use axum::http::StatusCode;
use models::{api::workspace::container_registry::*, prelude::*};

use crate::prelude::*;

pub async fn get_repository_manifest_details(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path:
					GetContainerRepositoryManifestDetailsPath {
						workspace_id: _,
						repository_id,
						digest_or_tag,
					},
				query: (),
				headers:
					GetContainerRepositoryManifestDetailsRequestHeaders {
						user_agent: _,
						authorization: _,
					},
				body: GetContainerRepositoryManifestDetailsRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		user_data: _,
		state: _,
	}: AuthenticatedAppRequest<'_, GetContainerRepositoryManifestDetailsRequest>,
) -> Result<AppResponse<GetContainerRepositoryManifestDetailsRequest>, ErrorType> {
	info!("Starting: Get manifest details");

	let (digest, platform, manifest_created) = query!(
		r#"
		SELECT
			repository_manifest.manifest_digest,
			COALESCE(image.os || '/' || image.architecture, 'unknown') AS "platform!",
			repository_manifest.created_at
		FROM
			container_registry_repository_manifest repository_manifest
		INNER JOIN
			container_registry_manifest manifest
		ON
			repository_manifest.manifest_digest = manifest.digest
		LEFT JOIN
			container_registry_manifest_image image
		ON
			image.manifest_digest = manifest.digest
		WHERE
			repository_manifest.repository_id = $1
			AND (
				repository_manifest.manifest_digest = $2 OR
				repository_manifest.manifest_digest = (
					SELECT
						manifest_digest
					FROM
						container_registry_repository_tag
					WHERE
						repository_id = $1 AND
						name = $2
				)
			)
		ORDER BY
			CASE
				WHEN repository_manifest.manifest_digest = $2 THEN 0
				ELSE 1
			END
		LIMIT 1;
		"#,
		repository_id as _,
		digest_or_tag as _
	)
	.fetch_optional(&mut **database)
	.await?
	.map(|manifest| {
		(
			manifest.manifest_digest,
			manifest.platform,
			manifest.created_at,
		)
	})
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	let tags = query!(
		r#"
		SELECT
			name,
			last_updated
		FROM
			container_registry_repository_tag
		WHERE
			repository_id = $1 AND
			manifest_digest = $2;
		"#,
		repository_id as _,
		digest as _
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|row| row.name)
	.collect();

	let size = query!(
		r#"
		SELECT
			(
				manifest.size +
				COALESCE(config_blob.size, 0) +
				COALESCE(layer_size.total_size, 0)
			)::BIGINT AS "manifest_size!"
		FROM
			container_registry_manifest manifest
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
					layer.manifest_digest = manifest.digest
		) layer_size
		ON
			TRUE
		WHERE
			manifest.digest = $1;
		"#,
		digest as _
	)
	.fetch_one(&mut **database)
	.await
	.map(|repo| repo.manifest_size as u64)?;

	let referenced_manifests = query!(
		r#"
		SELECT
			referenced_manifest.digest AS "digest",
			COALESCE(manifest_reference.os || '/' || manifest_reference.architecture, 'unknown') AS "platform!",
			(
				referenced_manifest.size +
				COALESCE(config_blob.size, 0) +
				COALESCE(layer_size.total_size, 0)
			)::BIGINT AS "size!",
			repository_manifest.created_at AS "created",
			COALESCE(
				(
					SELECT
						ARRAY_AGG(tag.name ORDER BY tag.last_updated DESC)
					FROM
						container_registry_repository_tag tag
					WHERE
						tag.repository_id = $1 AND
						tag.manifest_digest = referenced_manifest.digest
				),
				ARRAY[]::TEXT[]
			) AS "tags!: Vec<String>"
		FROM
			container_registry_manifest_reference manifest_reference
		INNER JOIN
			container_registry_manifest referenced_manifest
		ON
			referenced_manifest.digest = manifest_reference.referenced_digest
		LEFT JOIN
			container_registry_repository_manifest repository_manifest
		ON
			repository_manifest.repository_id = $1 AND
			repository_manifest.manifest_digest = referenced_manifest.digest
		LEFT JOIN
			container_registry_manifest_image ref_image
		ON
			ref_image.manifest_digest = referenced_manifest.digest
		LEFT JOIN
			container_registry_blob config_blob
		ON
			config_blob.digest = ref_image.config_blob_digest
		LEFT JOIN LATERAL (
			SELECT
				COALESCE(SUM(layer.size), 0)::BIGINT AS total_size
				FROM
					container_registry_manifest_layer layer
				WHERE
					layer.manifest_digest = referenced_manifest.digest
		) layer_size
		ON
			TRUE
		WHERE
			manifest_reference.manifest_digest = $2
		ORDER BY
			repository_manifest.created_at DESC;
		"#,
		repository_id as _,
		digest as _,
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|manifest| ContainerRepositoryManifestInfo {
		digest: manifest.digest,
		size: manifest.size as u64,
		platform: manifest.platform,
		created: manifest.created,
		tags: manifest.tags,
	})
	.collect();

	AppResponse::builder()
		.body(GetContainerRepositoryManifestDetailsResponse {
			manifest_details: ContainerRepositoryManifestInfo {
				digest,
				size,
				created: manifest_created,
				tags,
				platform,
			},
			referenced_manifests,
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
