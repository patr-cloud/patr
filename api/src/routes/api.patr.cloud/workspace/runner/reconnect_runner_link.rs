use argon2::{Algorithm, Argon2, PasswordHasher, Version, password_hash::generate_salt};
use axum::http::StatusCode;
use models::api::workspace::runner::*;
use rustis::commands::StringCommands;
use time::OffsetDateTime;

use crate::{
	models::redis::{RunnerApprovedSetupData, RunnerSetupDataEntry},
	prelude::*,
};

pub async fn reconnect_runner_link(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path:
					ReconnectRunnerLinkPath {
						workspace_id,
						user_code,
						runner_id,
					},
				query: (),
				headers:
					ReconnectRunnerLinkRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: ReconnectRunnerLinkRequestProcessed,
			},
		database,
		redis,
		client_ip: _,
		user_data: _,
		state,
	}: AuthenticatedAppRequest<'_, ReconnectRunnerLinkRequest>,
) -> Result<AppResponse<ReconnectRunnerLinkRequest>, ErrorType> {
	let key = redis::keys::runner_setup_data(workspace_id, &user_code);

	let Some(raw) = redis.get::<Option<String>>(&key).await? else {
		return Err(ErrorType::ResourceDoesNotExist);
	};
	let entry = serde_json::from_str::<RunnerSetupDataEntry>(&raw)?;

	if entry.approved.is_some() {
		// Already claimed by someone (or this user in another tab). The CLI will
		// pick up the existing credentials on its next verify poll.
		return Err(ErrorType::ResourceAlreadyExists);
	}

	// Bind the requested runner to its service account. Auth has already proven
	// the caller may regenerate this runner's token; this resolves which SA to
	// rotate and confirms the runner lives in this workspace.
	let Some(sa_id) = query!(
		r#"
		SELECT
			service_account_id AS "service_account_id: Uuid"
		FROM
			runner
		WHERE
			id = $1 AND
			workspace_id = $2;
		"#,
		runner_id as _,
		workspace_id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	.map(|row| row.service_account_id) else {
		return Err(ErrorType::ResourceDoesNotExist);
	};

	// Rotate the SA token. The old token stops authenticating on its next use.
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

	let rows_affected = query!(
		r#"
		UPDATE
			service_account
		SET
			token_hash = $1
		WHERE
			id = $2 AND
			deleted IS NULL;
		"#,
		&token_hash,
		sa_id as _,
	)
	.execute(&mut **database)
	.await?
	.rows_affected();

	if rows_affected == 0 {
		return Err(ErrorType::ResourceDoesNotExist);
	}

	// Invalidate cached permissions for the old token.
	redis
		.setex(
			redis::keys::user_id_revocation_timestamp(&sa_id),
			constants::CACHED_PERMISSIONS_VALIDITY
				.whole_seconds()
				.unsigned_abs(),
			OffsetDateTime::now_utc().unix_timestamp_nanos().to_string(),
		)
		.await?;

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
		.body(ReconnectRunnerLinkResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
