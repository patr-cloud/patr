use axum::http::StatusCode;
use models::api::workspace::*;

use crate::prelude::*;

/// The handler to create a new workspace. The workspace name must be unique.
pub async fn create_workspace(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: CreateWorkspacePath,
				query: (),
				headers: CreateWorkspaceRequestHeaders {
					authorization,
					user_agent,
				},
				body: CreateWorkspaceRequestProcessed { name },
			},
		database,
		redis,
		client_ip,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, CreateWorkspaceRequest>,
) -> Result<AppResponse<CreateWorkspaceRequest>, ErrorType> {
	info!("Creating workspace: `{name}`");

	let user_id = user_data.id;
	let available = super::is_name_available(AuthenticatedAppRequest {
		request: ProcessedApiRequest {
			path: IsWorkspaceNameAvailablePath,
			query: IsWorkspaceNameAvailableQuery {
				name: name.to_string(),
			},
			headers: IsWorkspaceNameAvailableRequestHeaders {
				authorization,
				user_agent,
			},
			body: IsWorkspaceNameAvailableRequestProcessed,
		},
		client_ip,
		config,
		database,
		redis,
		user_data,
	})
	.await?
	.body
	.available;

	if !available {
		return Err(ErrorType::WorkspaceNameAlreadyExists);
	}

	query!(
		r#"
		SET CONSTRAINTS ALL DEFERRED;
		"#
	)
	.execute(&mut **database)
	.await?;

	// Create resource
	let workspace_id = query!(
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
				(SELECT id FROM resource_type WHERE name = 'workspace'),
				gen_random_uuid(),
				NOW()
			)
		RETURNING id;
		"#,
	)
	.fetch_one(&mut **database)
	.await
	.map_err(|e| match e {
		sqlx::Error::Database(dbe) if dbe.is_unique_violation() => ErrorType::ResourceAlreadyExists,
		other => other.into(),
	})?
	.id;

	// Create new workspace in database
	query!(
		r#"
		INSERT INTO 
			workspace(
				id,
				name,
				super_admin_id,
				deleted
			)
		VALUES
			($1, $2, $3, NULL);
		"#,
		workspace_id as _,
		&name,
		user_id as _,
	)
	.execute(&mut **database)
	.await?;

	// Update resource's owner to workspace id
	query!(
		r#"
		UPDATE
			resource
		SET
			owner_id = $1
		WHERE
			id = $2;
		"#,
		workspace_id as _,
		workspace_id as _,
	)
	.execute(&mut **database)
	.await?;

	query!(
		r#"
		SET CONSTRAINTS ALL IMMEDIATE;
		"#
	)
	.execute(&mut **database)
	.await?;

	AppResponse::builder()
		.body(CreateWorkspaceResponse {
			id: WithId::from(workspace_id),
		})
		.headers(())
		.status_code(StatusCode::CREATED)
		.build()
		.into_result()
}
