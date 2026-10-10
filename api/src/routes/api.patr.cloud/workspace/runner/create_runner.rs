use axum::http::StatusCode;
#[cfg(feature = "cloud")]
use cloudflare::{
	endpoints::workerskv::write_key,
	framework::{
		Environment,
		auth::Credentials,
		client::{ClientConfig, async_api::Client as CloudflareClient},
	},
};
#[cfg(feature = "cloud")]
use models::cloudflare::kv::*;
use models::{api::workspace::runner::*, prelude::*};
use sha2::{Digest as _, Sha256};

use crate::{models::permissions, prelude::*};

pub async fn create_runner(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: CreateRunnerPath { workspace_id },
				query: (),
				headers:
					CreateRunnerRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: CreateRunnerRequestProcessed { name },
			},
		database,
		redis: _,
		client_ip: _,
		actor_data,
		state,
	}: AuthenticatedAppRequest<'_, CreateRunnerRequest>,
) -> Result<AppResponse<CreateRunnerRequest>, ErrorType> {
	info!("Creating runner with name: `{name}`");

	// Reject a taken name before writing anything. The runner insert below is
	// also guarded against the unique violation, for two creates racing past
	// this check.
	let name_taken = query!(
		r#"
		SELECT
			id
		FROM
			runner
		WHERE
			workspace_id = $1 AND
			name = $2 AND
			deleted IS NULL;
		"#,
		workspace_id as _,
		name.as_ref(),
	)
	.fetch_optional(&mut **database)
	.await?
	.is_some();

	if name_taken {
		return Err(ErrorType::ResourceAlreadyExists);
	}

	// Runner resource row
	let runner_id = query!(
		r#"
		INSERT INTO
			resource(
				id,
				resource_type_id,
				workspace_id,
				created
			)
		VALUES
			(
				GENERATE_RESOURCE_ID(),
				(SELECT id FROM resource_type WHERE name = 'runner'),
				$1,
				NOW()
			)
		RETURNING id AS "id: Uuid";
		"#,
		workspace_id as _,
	)
	.fetch_one(&mut **database)
	.await?
	.id;

	// Service account: resource row + service_account row
	let token = permissions::generate_service_account_token();
	let token_hash = hex::encode(Sha256::digest(&token));

	let sa_id = query!(
		r#"
		INSERT INTO
			resource(
				id,
				resource_type_id,
				workspace_id,
				created
			)
		VALUES
			(
				GENERATE_RESOURCE_ID(),
				(SELECT id FROM resource_type WHERE name = 'serviceAccount'),
				$1,
				NOW()
			)
		RETURNING id AS "id: Uuid";
		"#,
		workspace_id as _,
	)
	.fetch_one(&mut **database)
	.await?
	.id;

	// The same id registers the account as a client and as an actor, so the
	// bindings below can hang straight off it.
	query!(
		r#"
		INSERT INTO
			actor_client(
				id,
				actor_client_type
			)
		VALUES
			($1, 'service_account');
		"#,
		sa_id as _,
	)
	.execute(&mut **database)
	.await?;

	query!(
		r#"
		INSERT INTO
			workspace_actor(
				id,
				workspace_id,
				actor_type
			)
		VALUES
			($1, $2, 'service_account');
		"#,
		sa_id as _,
		workspace_id as _,
	)
	.execute(&mut **database)
	.await?;

	query!(
		r#"
		INSERT INTO
			service_account(
				id,
				name,
				workspace_id,
				created,
				description,
				token_hash,
				is_immutable
			)
		VALUES
			($1, $2, $3, NOW(), $4, $5, TRUE);
		"#,
		sa_id as _,
		format!("runner-{runner_id}"),
		workspace_id as _,
		Some(format!("Service account for runner '{name}'")),
		&token_hash,
	)
	.execute(&mut **database)
	.await?;

	// Two grants, because a binding carries a single scope: the runner reads
	// across the whole workspace, but may only execute on itself. Both roles
	// are immutable defaults seeded with the workspace.
	for (role_name, scope_id) in [
		("Runner: All Resource Reader", workspace_id),
		("Runner: Execute", runner_id),
	] {
		query!(
			r#"
			INSERT INTO
				role_binding(
					id,
					workspace_id,
					actor_id,
					role_id,
					scope_id,
					created,
					created_by
				)
			VALUES
				(
					GEN_RANDOM_UUID(),
					$1,
					$2,
					(
						SELECT
							id
						FROM
							role
						WHERE
							workspace_id = $1 AND
							name = $3
					),
					$4,
					NOW(),
					$5
				);
			"#,
			workspace_id as _,
			sa_id as _,
			role_name,
			scope_id as _,
			actor_data.id as _,
		)
		.execute(&mut **database)
		.await?;
	}

	// Runner row, now that the SA exists for the FK. The Cloudflare tunnel is
	// created on the runner's first ingress token fetch, so a failure here
	// can't leave one behind.
	query!(
		r#"
		INSERT INTO
			runner(
				id,
				name,
				is_connected,
				workspace_id,
				cloudflare_tunnel_id,
				version,
				service_account_id
			)
		VALUES
			($1, $2, FALSE, $3, '', '0.0.0', $4);
		"#,
		runner_id as _,
		name.as_ref(),
		workspace_id as _,
		sa_id as _,
	)
	.execute(&mut **database)
	.await
	.map_err(|err| match err {
		sqlx::Error::Database(dbe) if dbe.is_unique_violation() => ErrorType::ResourceAlreadyExists,
		err => err.into(),
	})?;

	// Cloudflare KV registers the runner so the worker routes to it
	cfg_if! {
		if #[cfg(feature = "cloud")] {
			CloudflareClient::new(
				Credentials::UserAuthToken {
					token: state.config.cloudflare.api_key.clone(),
				},
				ClientConfig::default(),
				Environment::Custom(state.config.cloudflare.base_url.clone()),
			)?
			.request(&write_key::WriteKey {
				account_identifier: &state.config.cloudflare.account_id,
				namespace_identifier: &state.config.cloudflare.worker_namespace_id,
				key: &runner_id.to_string(),
				params: write_key::WriteKeyParams {
					expiration: None,
					expiration_ttl: None,
				},
				body: write_key::WriteKeyBody::Value(serde_json::to_vec(&InternalKVData::Runner)?),
			})
			.await?;
		} else {
			let _ = &state;
		}
	}

	AppResponse::builder()
		.body(CreateRunnerResponse {
			id: WithId::from(runner_id),
			token,
		})
		.headers(())
		.status_code(StatusCode::CREATED)
		.build()
		.into_result()
}
