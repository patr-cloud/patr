use std::{net::IpAddr, sync::OnceLock};

use serde::{Deserialize, Serialize};

/// Lazily initialized HTTP client for reuse across validation requests.
static REQWEST_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

/// The result of validating a Cloudflare Turnstile token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnstileValidationResult {
	/// Whether the token was successfully validated.
	pub success: bool,
	/// Error codes returned by Cloudflare if validation failed.
	#[serde(default)]
	pub error_codes: Vec<String>,
	/// ISO timestamp of when the challenge was solved.
	#[serde(default)]
	pub challenge_ts: Option<String>,
	/// The action name configured for this Turnstile widget, if any.
	#[serde(default)]
	pub action: String,
}

/// Request body sent to Cloudflare's siteverify endpoint.
#[derive(Debug, Serialize)]
struct TurnstileVerifyRequest<'a> {
	/// The Turnstile secret key for your site.
	secret: &'a str,
	/// The token received from the client-side Turnstile widget.
	response: &'a str,
	/// IP address of the user who solved the challenge.
	remoteip: &'a str,
}

/// Validates a Cloudflare Turnstile token by calling the siteverify API.
pub async fn validate_turnstile_token(
	secret_key: &str,
	token: &str,
	remote_ip: IpAddr,
) -> Result<TurnstileValidationResult, reqwest::Error> {
	let remote_ip = remote_ip.to_string();
	let request_body = TurnstileVerifyRequest {
		secret: secret_key,
		response: token,
		remoteip: &remote_ip,
	};

	REQWEST_CLIENT
		.get_or_init(reqwest::Client::new)
		.post("https://challenges.cloudflare.com/turnstile/v0/siteverify")
		.form(&request_body)
		.send()
		.await?
		.json::<TurnstileValidationResult>()
		.await
}
