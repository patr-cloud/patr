use axum::http::StatusCode;
use models::api::workspace::runner::*;
use rustis::commands::StringCommands;
use sha2::{Digest as _, Sha256};

use crate::{
	models::{
		permissions,
		redis::{RunnerApprovedSetupData, RunnerSetupDataEntry},
	},
	prelude::*,
};

pub async fn reconnect_runner_link(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ReconnectRunnerLinkPath {
					workspace_id,
					user_code,
				},
				query: (),
				headers:
					ReconnectRunnerLinkRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: ReconnectRunnerLinkRequestProcessed { runner_id },
			},
		database,
		redis,
		client_ip: _,
		actor_data: _,
		state: _,
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
	let token = permissions::generate_service_account_token();
	let token_hash = hex::encode(Sha256::digest(&token));

	// `old` is the row as it was before the update (Postgres 18), so this hands
	// back the hash being replaced, whose cache entry has to go.
	let Some(old) = query!(
		r#"
		UPDATE
			service_account
		SET
			token_hash = $1
		WHERE
			id = $2 AND
			deleted IS NULL
		RETURNING
			old.token_hash;
		"#,
		&token_hash,
		sa_id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	else {
		return Err(ErrorType::ResourceDoesNotExist);
	};

	permissions::mark_token_stale(redis, &sa_id, &old.token_hash).await?;

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
					token,
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
