//! A minimal client for talking to OpenBao, the store for resource secrets
//! (deployment environment-variable secrets, etc.). OpenBao holds the secret
//! values; the API talks to it server-side, authenticating with the configured
//! token.

use std::fmt::Display;

use reqwest::Client;
use serde::Deserialize;

use crate::utils::config::OpenBaoConfig;

/// A client for talking to the OpenBao server. Holds the base endpoint and the
/// API token used to authenticate requests.
#[derive(Clone)]
pub struct OpenBaoClient {
	/// The HTTP client used to make requests to OpenBao.
	client: Client,
	/// The base URL of the OpenBao server, without a trailing slash.
	endpoint: String,
	/// The API token used to authenticate with OpenBao.
	token: String,
}

/// The status of the OpenBao server, used to verify that it is reachable,
/// unsealed, and that the configured token is valid.
#[derive(Debug)]
pub struct OpenBaoStatus {
	/// Whether the OpenBao server has been initialized.
	pub initialized: bool,
	/// Whether the OpenBao server is currently sealed.
	pub sealed: bool,
	/// Whether the configured API token was accepted by OpenBao.
	pub token_valid: bool,
}

/// The subset of OpenBao's `/v1/sys/health` response that we care about.
#[derive(Debug, Deserialize)]
struct HealthResponse {
	/// Whether the server has been initialized.
	initialized: bool,
	/// Whether the server is currently sealed.
	sealed: bool,
}

impl OpenBaoClient {
	/// Builds a new [`OpenBaoClient`] from the given configuration.
	pub fn new(config: &OpenBaoConfig) -> Self {
		Self {
			client: Client::new(),
			endpoint: config.endpoint.trim_end_matches('/').to_string(),
			token: config.token.clone(),
		}
	}

	/// Checks the status of the OpenBao server: whether it is reachable and
	/// unsealed, and whether the configured token is accepted. Used at startup
	/// to verify that the secret-store setup is correct.
	pub async fn status(&self) -> Result<OpenBaoStatus, reqwest::Error> {
		// `/v1/sys/health` is unauthenticated and returns a JSON body in every
		// state (including sealed / uninitialized, which use non-2xx statuses),
		// so we parse the body rather than treating a non-2xx as an error.
		let health = self
			.client
			.get(format!("{}/v1/sys/health", self.endpoint))
			.send()
			.await?
			.json::<HealthResponse>()
			.await?;

		// A valid token returns 200 from `lookup-self`; an invalid one returns
		// 403. A transport error still propagates via `?`.
		let token_valid = self
			.client
			.get(format!("{}/v1/auth/token/lookup-self", self.endpoint))
			.header("X-Vault-Token", &self.token)
			.send()
			.await?
			.status()
			.is_success();

		Ok(OpenBaoStatus {
			initialized: health.initialized,
			sealed: health.sealed,
			token_valid,
		})
	}

	/// Writes (creates or overwrites) a secret value in OpenBao's KV v2 store at
	/// `secret/data/{workspace_id}/{secret_id}`.
	pub async fn write_secret(
		&self,
		workspace_id: impl Display,
		secret_id: impl Display,
		value: &str,
	) -> Result<(), reqwest::Error> {
		self.client
			.post(format!(
				"{}/v1/secret/data/{}/{}",
				self.endpoint, workspace_id, secret_id
			))
			.header("X-Vault-Token", &self.token)
			.json(&serde_json::json!({ "data": { "value": value } }))
			.send()
			.await?
			.error_for_status()?;

		Ok(())
	}

	/// Permanently deletes a secret (all versions) from OpenBao's KV v2 store by
	/// removing its metadata at `secret/metadata/{workspace_id}/{secret_id}`.
	pub async fn delete_secret(
		&self,
		workspace_id: impl Display,
		secret_id: impl Display,
	) -> Result<(), reqwest::Error> {
		self.client
			.delete(format!(
				"{}/v1/secret/metadata/{}/{}",
				self.endpoint, workspace_id, secret_id
			))
			.header("X-Vault-Token", &self.token)
			.send()
			.await?
			.error_for_status()?;

		Ok(())
	}
}
