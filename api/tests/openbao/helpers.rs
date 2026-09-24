use base64::prelude::*;
use models::utils::Uuid;

/// Build a Basic Authorization header value for runner auth.
pub fn basic_auth(runner_id: &Uuid, api_token: &str) -> String {
	format!(
		"Basic {}",
		BASE64_STANDARD.encode(format!("{}:{}", runner_id, api_token))
	)
}

/// The OpenBao KV v2 read path for a secret.
pub fn secret_path(workspace_id: &Uuid, secret_id: &Uuid) -> String {
	format!("/v1/secret/data/{}/{}", workspace_id, secret_id)
}
