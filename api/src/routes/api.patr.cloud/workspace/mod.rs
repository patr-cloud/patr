use axum::{http::StatusCode, Router};
use models::api::workspace::*;

use crate::prelude::*;

// mod container_registry;
mod database;
mod deployment;
// mod domain;
mod managed_url;
mod rbac;
mod runner;
mod secret;
// mod static_site;

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		// .merge(container_registry::setup_routes(state).await)
		.merge(database::setup_routes(state).await)
		.merge(deployment::setup_routes(state).await)
		// .merge(domain::setup_routes(state).await)
		.merge(managed_url::setup_routes(state).await)
		.merge(rbac::setup_routes(state).await)
		.merge(runner::setup_routes(state).await)
		.merge(secret::setup_routes(state).await)
		// .merge(static_site::setup_routes(state).await)
		.mount_auth_endpoint(create_workspace, state)
}

async fn create_workspace(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: CreateWorkspacePath,
				query: (),
				headers:
					CreateWorkspaceRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: CreateWorkspaceRequestProcessed { workspace_name },
			},
		database,
		redis,
		client_ip: _,
		config: _,
		user_data,
	}: AuthenticatedAppRequest<'_, CreateWorkspaceRequest>,
) -> Result<AppResponse<CreateWorkspaceRequest>, ErrorType> {
	info!("Creating workspace: `{workspace_name}`",);

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
	.id
	.into();

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
		&workspace_name,
		user_data.id as _,
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
		.body(CreateWorkspaceResponse { workspace_id })
		.headers(())
		.status_code(StatusCode::CREATED)
		.build()
		.into_result()
}
