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
		mut state,
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
		workspace_id,
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
		RETURNING id;
		"#,
		workspace_id,
	)
	.fetch_one(&mut **database)
	.await?
	.id;

	// The same id registers the runner as an actor (the accountable identity)
	// and as an actor_client that it owns itself, so its requests can be
	// audited. Both rows outlive the runner, since `audit_log` points at them.
	query!(
		r#"
		INSERT INTO
			actor(
				id,
				actor_type,
				created
			)
		VALUES
			($1, 'runner', NOW());
		"#,
		runner_id,
	)
	.execute(&mut **database)
	.await?;

	query!(
		r#"
		INSERT INTO
			actor_client(
				id,
				actor_client_type,
				actor_id
			)
		VALUES
			($1, 'runner', $1);
		"#,
		runner_id,
	)
	.execute(&mut **database)
	.await?;

	let token = permissions::generate_runner_token();
	let token_hash = hex::encode(Sha256::digest(&token));

	// Runner row, now that its actor and client exist for the FKs. The
	// Cloudflare tunnel is created on the runner's first ingress token fetch,
	// so a failure here can't leave one behind.
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
				token_hash
			)
		VALUES
			($1, $2, FALSE, $3, '', '0.0.0', $4);
		"#,
		runner_id,
		name.as_ref(),
		workspace_id,
		&token_hash,
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
		}
	}

	let super_admin = query!(
		r#"
		SELECT
			workspace.name::TEXT AS "workspace_name!",
			"user".email::TEXT AS "email!",
			"user".first_name
		FROM
			workspace
		INNER JOIN
			"user"
		ON
			"user".id = workspace.super_admin_id
		WHERE
			workspace.id = $1;
		"#,
		workspace_id,
	)
	.fetch_one(&mut **database)
	.await?;

	// Cloud serves the dashboard on `app.`; self-hosted off the base domain.
	let base_domain = &state.config.server.base_domain;
	let dashboard_url = if cfg!(feature = "cloud") {
		format!("https://app.{base_domain}")
	} else {
		format!("https://{base_domain}")
	};

	state
		.worker
		.send_email(
			super_admin.email,
			RunnerCreatedEmail {
				first_name: super_admin.first_name,
				runner_name: name.into_owned(),
				workspace_name: super_admin.workspace_name,
				created_by: actor_data.actor.display_name(),
				runner_url: format!("{dashboard_url}/runners/{runner_id}"),
			},
		)
		.await
		.inspect_err(|err| {
			error!("Error enqueuing runner created email: `{err}`");
		})?;

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
