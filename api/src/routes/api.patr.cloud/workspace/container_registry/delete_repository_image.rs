use axum::http::StatusCode;
use models::{api::workspace::container_registry::*, prelude::*};

use crate::prelude::*;

pub async fn delete_repository_image(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path:
					DeleteContainerRepositoryImagePath {
						workspace_id,
						repository_id,
						digest,
					},
				query: (),
				headers:
					DeleteContainerRepositoryImageRequestHeaders {
						user_agent: _,
						authorization: _,
					},
				body: DeleteContainerRepositoryImageRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, DeleteContainerRepositoryImageRequest>,
) -> Result<AppResponse<DeleteContainerRepositoryImageRequest>, ErrorType> {
	info!("Starting: Delete container repository image");

	// Get repository detail
	let repository_name = query!(
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

	let name = format!("{}/{}", workspace_id, repository_name);

	// Delete all tags for the given image
	let container_repo_tag_info: Vec<ContainerRepositoryTagInfo> = query!(
		r#"
		SELECT
			tag,
			last_updated
		FROM
			container_registry_repository_tag
		WHERE
			repository_id = $1 AND
			manifest_digest = $2;
		"#,
		repository_id as _,
		digest
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|row| ContainerRepositoryTagInfo {
		tag: row.tag,
		last_updated: row.last_updated.into(),
	})
	.collect();

	for tag in container_repo_tag_info {
		query!(
			r#"
			DELETE FROM
				container_registry_repository_tag
			WHERE
				repository_id = $1 AND
				tag = $2;
			"#,
			repository_id as _,
			tag.tag
		)
		.execute(&mut **database)
		.await?;
	}

	// Delete container repository image with digest from database
	query!(
		r#"
		DELETE FROM
			container_registry_repository_manifest
		WHERE
			repository_id = $1 AND
			manifest_digest = $2;
		"#,
		repository_id as _,
		digest
	)
	.execute(&mut **database)
	.await?;

	// Update storage used after deleting in usage history
	let total_storage = query!(
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
	.map(|repo| repo.size)?;

	todo!("Update usage history with the new size");

	// Delete container repository in registry
	todo!("Is god user's ID required or is current user ID okay?");

	super::delete_docker_repository_image_in_registry(&name, &user_data.username, &digest, &config)
		.await?;

	AppResponse::builder()
		.body(DeleteContainerRepositoryImageResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
