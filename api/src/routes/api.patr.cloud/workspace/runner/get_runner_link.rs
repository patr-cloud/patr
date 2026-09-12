use axum::http::StatusCode;
use models::api::workspace::runner::*;
use rustis::commands::StringCommands;

use crate::{models::redis::RunnerSetupDataEntry, prelude::*};

pub async fn get_runner_link(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: GetRunnerLinkPath {
					workspace_id,
					user_code,
				},
				query: (),
				headers:
					GetRunnerLinkRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: GetRunnerLinkRequestProcessed,
			},
		database: _,
		redis,
		client_ip: _,
		user_data: _,
		state: _,
	}: AuthenticatedAppRequest<'_, GetRunnerLinkRequest>,
) -> Result<AppResponse<GetRunnerLinkRequest>, ErrorType> {
	let Some(raw) = redis
		.get::<Option<String>>(redis::keys::runner_setup_data(workspace_id, &user_code))
		.await?
	else {
		return Err(ErrorType::ResourceDoesNotExist);
	};

	let entry = serde_json::from_str::<RunnerSetupDataEntry>(&raw)?;

	AppResponse::builder()
		.body(GetRunnerLinkResponse {
			version: entry.version,
			os: entry.os,
			arch: entry.arch,
			hostname: entry.hostname,
			public_ip: entry.public_ip,
			private_ip: entry.private_ip,
			city: entry.city,
			country: entry.country,
			latitude: entry.latitude,
			longitude: entry.longitude,
			created_at: entry.created_at,
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
