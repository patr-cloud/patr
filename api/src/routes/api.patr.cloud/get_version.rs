use axum::http::StatusCode;
use models::api::*;

use crate::prelude::*;

pub async fn get_version(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: GetVersionPath,
				query: (),
				headers: GetVersionRequestHeaders { user_agent: _ },
				body: GetVersionRequestProcessed,
			},
		database: _,
		redis: _,
		client_ip: _,
		state: _,
	}: AppRequest<'_, GetVersionRequest>,
) -> Result<AppResponse<GetVersionRequest>, ErrorType> {
	AppResponse::builder()
		.body(GetVersionResponse {
			// CI bakes the channel's full version (e.g. `0.18.0-alpha.1`) into
			// PATR_BUILD_VERSION; Cargo.toml only ever holds the bare release.
			version: option_env!("PATR_BUILD_VERSION")
				.unwrap_or(env!("CARGO_PKG_VERSION"))
				.parse()
				.expect("PATR_BUILD_VERSION must be a valid semver"),
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
