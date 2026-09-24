use models::utils::constants;

use crate::{
	prelude::*,
	utils::client::{REQUEST_CLIENT, initialize_client},
};

/// The subset of OpenBao's KV v2 read response that we care about: the value
/// sits at `data.data.value`.
#[derive(serde::Deserialize)]
struct ReadSecretResponse {
	/// The KV v2 envelope, holding the stored data and its metadata.
	data: ReadSecretResponseData,
}

/// The `data` envelope of a KV v2 read response.
#[derive(serde::Deserialize)]
struct ReadSecretResponseData {
	/// The stored key-value pairs.
	data: ReadSecretResponseValue,
}

/// The key-value pairs stored at a secret's path.
#[derive(serde::Deserialize)]
struct ReadSecretResponseValue {
	/// The secret value itself.
	value: String,
}

/// Read a secret's value from `openbao.patr.cloud`, which proxies OpenBao.
///
/// This talks OpenBao's own KV v2 API rather than a Patr endpoint, and
/// authenticates the way the log and metric proxies do: Basic auth carrying
/// `{runner_id}:{api_token}`. The value is never logged.
#[instrument(skip_all, fields(secret_id = %secret_id))]
pub async fn get_secret_value(
	runner_id: Uuid,
	api_token: &BearerToken,
	workspace_id: Uuid,
	secret_id: Uuid,
) -> Result<String, ErrorType> {
	let response = REQUEST_CLIENT
		.get_or_init(initialize_client)
		.get(format!(
			"{}/v1/secret/data/{}/{}",
			constants::OPENBAO_BASE_URL,
			workspace_id,
			secret_id
		))
		.basic_auth(runner_id.to_string(), Some(api_token.0.token()))
		.send()
		.await
		.map_err(ErrorType::server_error)?;

	if !response.status().is_success() {
		return Err(ErrorType::server_error(format!(
			"secrets returned {} for secret `{}`",
			response.status(),
			secret_id
		)));
	}

	Ok(response
		.json::<ReadSecretResponse>()
		.await
		.map_err(ErrorType::server_error)?
		.data
		.data
		.value)
}
