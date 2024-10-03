use axum::http::StatusCode;
use models::{api::workspace::runner::*, prelude::*};

use crate::prelude::*;

pub async fn add_runner_to_workspace(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: AddRunnerToWorkspacePath { workspace_id },
				query: (),
				headers:
					AddRunnerToWorkspaceRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: AddRunnerToWorkspaceRequestProcessed { name },
			},
		database,
		redis: _,
		client_ip: _,
		config: _,
		user_data: _,
	}: AuthenticatedAppRequest<'_, AddRunnerToWorkspaceRequest>,
) -> Result<AppResponse<AddRunnerToWorkspaceRequest>, ErrorType> {
	info!("Creating Runner with name: `{name}`");

	let id = query!(
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
				(SELECT id FROM resource_type WHERE name = 'runner'),
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

	query!(
		r#"
		INSERT INTO
			runner(
				id,
				name,
				workspace_id,
				cloudflare_tunnel_id
			)
		VALUES
			(
				$1,
				$2,
				$3,
				'qwertyuiop'
			);
		"#,
		id as _,
		name.as_ref(),
		workspace_id as _,
	)
	.execute(&mut **database)
	.await?;

	AppResponse::builder()
		.body(AddRunnerToWorkspaceResponse {
			id: WithId::from(id),
		})
		.headers(())
		.status_code(StatusCode::CREATED)
		.build()
		.into_result()
}
