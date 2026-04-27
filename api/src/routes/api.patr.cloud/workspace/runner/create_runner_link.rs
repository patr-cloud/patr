use axum::http::StatusCode;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use models::api::workspace::runner::*;
use rand::{RngExt, distr::slice::Choose};
use rustis::commands::StringCommands;
use time::OffsetDateTime;

use crate::{
	models::{ip_lookup, redis::RunnerSetupDataEntry},
	prelude::*,
};

pub async fn create_runner_link(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: CreateRunnerLinkPath { workspace_id },
				query: (),
				headers:
					CreateRunnerLinkRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body:
					CreateRunnerLinkRequestProcessed {
						version,
						os,
						arch,
						hostname,
						private_ip,
					},
			},
		database: _,
		redis,
		client_ip,
		user_data: _,
		state,
	}: AuthenticatedAppRequest<'_, CreateRunnerLinkRequest>,
) -> Result<AppResponse<CreateRunnerLinkRequest>, ErrorType> {
	const DEVICE_CODE_BYTES: usize = 32;
	const USER_CODE_LENGTH: usize = 8;
	/// Base32-ish alphabet without ambiguous characters (no `0/O`, `1/I/L`).
	const USER_CODE_ALPHABET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";

	let (device_code, user_code) = {
		let mut rng = rand::rng();

		let device_code = URL_SAFE_NO_PAD.encode({
			let mut device_code_bytes = [0u8; DEVICE_CODE_BYTES];
			rng.fill(&mut device_code_bytes);
			device_code_bytes
		});

		let user_code = rng
			.sample_iter(Choose::new(USER_CODE_ALPHABET).unwrap())
			.take(USER_CODE_LENGTH)
			.map(|&b| b as char)
			.collect::<String>();

		(device_code, user_code)
	};

	let ip_details = ip_lookup::lookup(client_ip, &state).await.ok();

	let (latitude, longitude) = ip_details
		.as_ref()
		.and_then(|d| d.loc.split_once(','))
		.and_then(|(lat, lon)| {
			Some((
				lat.trim().parse::<f64>().ok()?,
				lon.trim().parse::<f64>().ok()?,
			))
		})
		.map_or((None, None), |(lat, lon)| (Some(lat), Some(lon)));

	let entry = RunnerSetupDataEntry {
		device_code: device_code.clone(),
		version,
		os,
		arch,
		hostname,
		public_ip: client_ip,
		private_ip,
		city: ip_details.as_ref().map(|d| d.city.clone()),
		country: ip_details.as_ref().map(|d| d.country.clone()),
		latitude,
		longitude,
		created_at: OffsetDateTime::now_utc(),
		approved: None,
	};

	redis
		.setex(
			redis::keys::runner_setup_data(workspace_id, &user_code),
			constants::RUNNER_LINK_VALIDITY
				.whole_seconds()
				.unsigned_abs(),
			serde_json::to_string(&entry)?,
		)
		.await?;

	// Verification URL doesn't include workspace_id — the browser uses the
	// user's currently-selected workspace from the app context. If they're in
	// the wrong one the API returns 404, and the consent page nudges them to
	// switch workspaces.
	let verification_uri = format!("{}/runner/setup", constants::FRONTEND_BASE_URL);
	let verification_uri_complete = format!("{verification_uri}?code={user_code}");

	AppResponse::builder()
		.body(CreateRunnerLinkResponse {
			user_code,
			device_code,
			verification_uri,
			verification_uri_complete,
			expires_in: constants::RUNNER_LINK_VALIDITY
				.whole_seconds()
				.unsigned_abs(),
			interval: constants::RUNNER_LINK_POLL_INTERVAL
				.whole_seconds()
				.unsigned_abs(),
		})
		.headers(())
		.status_code(StatusCode::CREATED)
		.build()
		.into_result()
}
