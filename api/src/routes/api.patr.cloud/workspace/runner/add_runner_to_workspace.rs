use axum::http::StatusCode;
use cloudflare::{
	endpoints::{cfd_tunnel::*, workerskv::write_key},
	framework::{
		Environment,
		auth::Credentials,
		client::{
			ClientConfig,
			async_api::{Client, Client as CloudflareClient},
		},
	},
};
use models::{api::workspace::runner::*, cloudflare::kv::*, prelude::*};

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
				is_connected,
				workspace_id,
				cloudflare_tunnel_id
			)
		VALUES
			(
				$1,
				$2,
				FALSE,
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

	CloudflareClient::new(
		Credentials::UserAuthToken {
			token: state.config.cloudflare.api_key.clone(),
		},
		ClientConfig::default(),
		Environment::Production,
	)?
	.request(&write_key::WriteKey {
		account_identifier: &state.config.cloudflare.account_id,
		namespace_identifier: &state.config.cloudflare.worker_namespace_id,
		key: &id.to_string(),
		params: write_key::WriteKeyParams {
			expiration: None,
			expiration_ttl: None,
		},
		body: write_key::WriteKeyBody::Value(serde_json::to_vec(&InternalKVData::Runner)?),
	})
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
