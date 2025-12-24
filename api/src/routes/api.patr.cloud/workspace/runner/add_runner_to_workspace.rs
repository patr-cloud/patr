use axum::http::StatusCode;
use cloudflare::{
	endpoints::cfd_tunnel::*,
	framework::{Environment, auth::Credentials, client::async_api::Client},
};
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
		user_data: _,
		state,
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

	let tunnel_id = Client::new(
		Credentials::UserAuthToken {
			token: state.config.cloudflare.api_key.clone(),
		},
		Default::default(),
		Environment::Production,
	)?
	.request(&create_tunnel::CreateTunnel {
		account_identifier: &state.config.cloudflare.account_id,
		params: create_tunnel::Params {
			config_src: &ConfigurationSrc::Cloudflare,
			name: &format!("Runner: {}", id),
			tunnel_secret: &b"default".to_vec(),
			metadata: None,
		},
	})
	.await?
	.result
	.id
	.to_string();

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
				$4
			);
		"#,
		id as _,
		name.as_ref(),
		workspace_id as _,
		tunnel_id,
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
