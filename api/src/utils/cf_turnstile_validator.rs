use std::{net::IpAddr, sync::OnceLock};

use reqwest::Client;
use serde::{Deserialize, Serialize};

/// Lazily initialized HTTP client for reuse across validation requests.
static REQWEST_CLIENT: OnceLock<Client> = OnceLock::new();

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
	/// Optional IP address of the user who solved the challenge.
	#[serde(skip_serializing_if = "Option::is_none")]
	remoteip: Option<&'a str>,
}

/// Validates a Cloudflare Turnstile token by calling the siteverify API.
///
/// # Arguments
///
/// * `secret_key` - Your Cloudflare Turnstile secret key.
/// * `token` - The token received from the client-side widget.
/// * `remote_ip` - Optional IP address of the user for additional verification.
///
/// # Returns
///
/// A [`TurnstileValidationResult`] indicating whether the token is valid.
///
/// # Errors
///
/// Returns a [`reqwest::Error`] if the HTTP request or response parsing fails.
pub async fn validate_turnstile_token(
	secret_key: &str,
	token: &str,
	remote_ip: Option<IpAddr>,
) -> Result<TurnstileValidationResult, reqwest::Error> {
	let remote_ip = remote_ip.map(|ip| ip.to_string());
	let request_body = TurnstileVerifyRequest {
		secret: secret_key,
		response: token,
		remoteip: remote_ip.as_deref(),
	};

	REQWEST_CLIENT
		.get_or_init(Client::new)
		.post("https://challenges.cloudflare.com/turnstile/v0/siteverify")
		.form(&request_body)
		.send()
		.await?
		.json::<TurnstileValidationResult>()
		.await
}
