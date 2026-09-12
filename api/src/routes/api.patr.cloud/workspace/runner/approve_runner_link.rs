use argon2::{Algorithm, Argon2, PasswordHasher, Version, password_hash::generate_salt};
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
use rustis::commands::StringCommands;

use crate::{
	models::redis::{RunnerApprovedSetupData, RunnerSetupDataEntry},
	prelude::*,
};

pub async fn approve_runner_link(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ApproveRunnerLinkPath {
					workspace_id,
					user_code,
				},
				query: (),
				headers:
					ApproveRunnerLinkRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: ApproveRunnerLinkRequestProcessed { runner_name },
			},
		database,
		redis,
		client_ip: _,
		user_data,
		state,
	}: AuthenticatedAppRequest<'_, ApproveRunnerLinkRequest>,
) -> Result<AppResponse<ApproveRunnerLinkRequest>, ErrorType> {
	let key = redis::keys::runner_setup_data(workspace_id, &user_code);

	let Some(raw) = redis.get::<Option<String>>(&key).await? else {
		return Err(ErrorType::ResourceDoesNotExist);
	};
	let entry = serde_json::from_str::<RunnerSetupDataEntry>(&raw)?;

	if entry.approved.is_some() {
		// Already approved by someone (or this user in another tab). The CLI
		// will pick up the existing credentials on its next verify poll.
		return Err(ErrorType::ResourceAlreadyExists);
	}

	// Reject a taken name up front. The insert below is also guarded against
	// the unique violation, but creating the Cloudflare tunnel happens first
	// and is an external side effect the transaction rollback cannot undo — so
	// the common duplicate-name case must never get that far.
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
		runner_name.as_ref(),
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
	let refresh_token = Uuid::new_v4();
	let token_hash = Argon2::new_with_secret(
		state.config.password_pepper.as_bytes(),
		Algorithm::Argon2id,
		Version::V0x13,
		constants::HASHING_PARAMS,
	)
	.map_err(ErrorType::server_error)?
	.hash_password_with_salt(refresh_token.as_bytes(), &generate_salt())
	.map_err(ErrorType::server_error)?
	.to_string();

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
				token_hash
			)
		VALUES
			($1, $2, $3, NOW(), $4, $5);
		"#,
		sa_id as _,
		format!("runner-{runner_id}"),
		workspace_id as _,
		Some(format!("Service account for runner '{runner_name}'")),
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
			user_data.id as _,
		)
		.execute(&mut **database)
		.await?;
	}

	// Cloudflare tunnel for the runner. Self-hosted has no Cloudflare account to
	// create one on, so the column is left empty there.
	cfg_if! {
		if #[cfg(feature = "cloud")] {
			let tunnel_id =
				utils::cloudflare::create_tunnel_with_config(runner_id, &state.config).await?;
		} else {
			let tunnel_id = String::new();
		}
	}

	// Runner row, now that the SA exists for the FK
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
			($1, $2, FALSE, $3, $4, '0.0.0', $5);
		"#,
		runner_id as _,
		runner_name.as_ref(),
		workspace_id as _,
		tunnel_id,
		sa_id as _,
	)
	.execute(&mut **database)
	.await
	.map_err(|err| match err {
		// Backstop for two approvals racing past the check above.
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

	// Mark the link approved in Redis. CLI's next verify poll picks this up and
	// one-shot deletes the entry.
	redis
		.setex(
			&key,
			constants::RUNNER_LINK_VALIDITY
				.whole_seconds()
				.unsigned_abs(),
			serde_json::to_string(&RunnerSetupDataEntry {
				approved: Some(RunnerApprovedSetupData {
					runner_id,
					workspace_id,
					token: format!("patrv1.{}.{}", refresh_token, sa_id),
				}),
				..entry
			})?,
		)
		.await?;

	AppResponse::builder()
		.body(ApproveRunnerLinkResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
