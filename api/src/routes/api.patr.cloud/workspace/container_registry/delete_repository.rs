use axum::http::StatusCode;
use models::{api::workspace::container_registry::*, prelude::*};

use crate::prelude::*;

pub async fn delete_repository(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path:
					DeleteContainerRepositoryPath {
						workspace_id: _,
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
		user_data: _,
		state: _,
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

	// Delete the repository
	query!(
		r#"
		DELETE FROM
			container_registry_repository
		WHERE
			id = $1;
		"#,
		repository_id as _
	)
	.execute(&mut **database)
	.await?;

	AppResponse::builder()
		.body(DeleteContainerRepositoryResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
