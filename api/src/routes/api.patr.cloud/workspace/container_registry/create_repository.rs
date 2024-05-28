use axum::http::StatusCode;
use models::{api::workspace::container_registry::*, prelude::*};

use crate::prelude::*;

pub async fn create_repository(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: CreateContainerRepositoryPath { workspace_id },
				query: (),
				headers:
					CreateContainerRepositoryRequestHeaders {
						user_agent: _,
						authorization: _,
					},
				body: CreateContainerRepositoryRequestProcessed { name },
			},
		database,
		redis: _,
		client_ip: _,
		config: _,
		user_data: _,
	}: AuthenticatedAppRequest<'_, CreateContainerRepositoryRequest>,
) -> Result<AppResponse<CreateContainerRepositoryRequest>, ErrorType> {
	info!(
		"Creating container registry repository: `{}` in workspaceId: `{}`",
		name, workspace_id
	);

	// Create resource
	let resource_id = query!(
		r#"
		INSERT INTO
			resource(
				id,
				resource_type_id,
				owner_id,
				created
			)
		VALUES
			(
				GENERATE_RESOURCE_ID(),
				(SELECT id FROM resource_type WHERE name = 'container_repository'),
				$1,
				NOW()
			)
		RETURNING id;
		"#,
		workspace_id as _,
	)
	.fetch_one(&mut **database)
	.await
	.map_err(|e| match e {
		sqlx::Error::Database(dbe) if dbe.is_unique_violation() => ErrorType::ResourceAlreadyExists,
		other => other.into(),
	})?
	.id;

	// Create new repository in database
	query!(
		r#"
		INSERT INTO 
			container_registry_repository(
				id,
				workspace_id,
				name,
                deleted
			)
		VALUES
			($1, $2, $3, NULL);
		"#,
		resource_id as _,
		workspace_id as _,
		name as _
	)
	.execute(&mut **database)
	.await?;

	AppResponse::builder()
		.body(CreateContainerRepositoryResponse {
			id: WithId::from(resource_id),
		})
		.headers(())
		.status_code(StatusCode::CREATED)
		.build()
		.into_result()
}
