use axum::http::StatusCode;
use constant_time_eq::constant_time_eq;
use models::api::workspace::runner::*;
use rustis::commands::{GenericCommands, StringCommands};

use crate::{models::redis::RunnerSetupDataEntry, prelude::*};

pub async fn verify_runner_link(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: VerifyRunnerLinkPath { workspace_id },
				query: (),
				headers:
					VerifyRunnerLinkRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: VerifyRunnerLinkRequestProcessed {
					user_code,
					device_code,
				},
			},
		database: _,
		redis,
		client_ip: _,
		user_data: _,
		state: _,
	}: AuthenticatedAppRequest<'_, VerifyRunnerLinkRequest>,
) -> Result<AppResponse<VerifyRunnerLinkRequest>, ErrorType> {
	let key = redis::keys::runner_setup_data(workspace_id, &user_code);

	let Some(raw) = redis.get::<Option<String>>(&key).await? else {
		// Either expired (TTL elapsed) or already claimed (one-shot delete).
		// Both look the same to the CLI on purpose — don't leak which one.
		return Err(ErrorType::ResourceDoesNotExist);
	};

	let entry = serde_json::from_str::<RunnerSetupDataEntry>(&raw)?;

	if !constant_time_eq(entry.device_code.as_bytes(), device_code.as_bytes()) {
		return Err(ErrorType::Unauthorized);
	}

	let Some(approved) = entry.approved else {
		return AppResponse::builder()
			.body(VerifyRunnerLinkResponse {
				result: VerifyRunnerLinkResult::Pending,
			})
			.headers(())
			.status_code(StatusCode::ACCEPTED)
			.build()
			.into_result();
	};

	// One-shot: delete the entry so the token can't be claimed twice.
	redis.del(&key).await?;

	AppResponse::builder()
		.body(VerifyRunnerLinkResponse {
			result: VerifyRunnerLinkResult::Approved {
				runner_id: approved.runner_id,
				workspace_id: approved.workspace_id,
				token: approved.token,
			},
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
