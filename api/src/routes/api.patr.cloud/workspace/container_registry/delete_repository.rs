use axum::http::StatusCode;
use models::{api::workspace::container_registry::*, prelude::*};
use time::OffsetDateTime;

use crate::prelude::*;

pub async fn delete_repository(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: DeleteContainerRepositoryPath {
					workspace_id,
					repository_id,
				},
				query: (),
				headers:
					DeleteContainerRepositoryRequestHeaders {
						user_agent: _,
						authorization: _,
					},
				body: DeleteContainerRepositoryRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, DeleteContainerRepositoryRequest>,
) -> Result<AppResponse<DeleteContainerRepositoryRequest>, ErrorType> {
	info!(
		"Deleting container registry repository: `{}`",
		repository_id
	);

	// Check if any deployment currently running the repository
	let repo_being_used = query!(
		r#"
		SELECT
			id
		FROM
			deployment
		WHERE
			repository_id = $1 AND
			status != 'deleted';
		"#,
		repository_id as _
	)
	.fetch_optional(&mut **database)
	.await?
	.is_some();

	if repo_being_used {
		return Err(ErrorType::ResourceInUse);
	}

	// Delete from container registry
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
	.ok_or(ErrorType::ResourceDoesNotExist)?
	.name;

	let name = format!("{}/{}", &workspace_id, repository_name);

	let images = query!(
		r#"
		SELECT
			manifest_digest
		FROM
			container_registry_repository_manifest
		WHERE
			repository_id = $1;
		"#,
		repository_id as _
	)
	.fetch_all(&mut **database)
	.await?;

	// Deleting all tags for the given repository
	query!(
		r#"
		DELETE FROM
			container_registry_repository_tag
		WHERE
			repository_id = $1;
		"#,
		repository_id as _
	)
	.execute(&mut **database)
	.await?;

	// Deleting all images for the given repository
	query!(
		r#"
		DELETE FROM
			container_registry_repository_manifest
		WHERE
			repository_id = $1;
		"#,
		repository_id as _
	)
	.execute(&mut **database)
	.await?;

	// Updating the name of docker repository to deleted
	query!(
		r#"
		UPDATE
			container_registry_repository
		SET
			deleted = $2
		WHERE
			id = $1;
		"#,
		repository_id as _,
		OffsetDateTime::now_utc()
	)
	.execute(&mut **database)
	.await?;

	for image in images {
		super::delete_docker_repository_image_in_registry(
			&name,
			&user_data.username,
			&image.manifest_digest,
			&config,
		)
		.await?;
	}

	AppResponse::builder()
		.body(DeleteContainerRepositoryResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
