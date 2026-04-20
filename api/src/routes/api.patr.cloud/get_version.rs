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
			// Parse CARGO_PKG_VERSION directly so pre-release labels
			// (e.g. `0.18.0-alpha.1`) are preserved — macros::version!()
			// only reads MAJOR/MINOR/PATCH and drops the pre-release.
			version: env!("CARGO_PKG_VERSION")
				.parse()
				.expect("CARGO_PKG_VERSION must be a valid semver"),
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
