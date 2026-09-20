use axum::http::StatusCode;
use models::api::workspace::secret::*;
use zeroize::Zeroize;

use crate::prelude::*;

/// Updates a secret: renames the metadata row and, when a new value is provided,
/// overwrites the value in OpenBao. An omitted value keeps the existing one. The
/// plaintext value never touches Postgres and is zeroized once written.
pub async fn update_secret(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: UpdateSecretPath {
					workspace_id,
					secret_id,
				},
				query: (),
				headers: _,
				body: UpdateSecretRequestProcessed { name, value },
			},
		database,
		redis: _,
		client_ip: _,
		user_data: _,
		state,
	}: AuthenticatedAppRequest<'_, UpdateSecretRequest>,
) -> Result<AppResponse<UpdateSecretRequest>, ErrorType> {
	trace!("Updating secret ID: `{secret_id}`");

	query!(
		r#"
		UPDATE
			secret
		SET
			name = $1,
			last_updated = NOW()
		WHERE
			id = $2;
		"#,
		name as _,
		secret_id as _,
	)
	.execute(&mut **database)
	.await?;

	// Only touch OpenBao when a new value was supplied; otherwise the existing
	// value is kept. Zeroize the plaintext regardless of the outcome.
	if let Some(mut value) = value {
		let write = reqwest::Client::new()
			.post(format!(
				"{}/v1/secret/data/{}/{}",
				state.config.open_bao.endpoint.trim_end_matches('/'),
				workspace_id,
				secret_id
			))
			.header("X-Vault-Token", &state.config.open_bao.token)
			.json(&serde_json::json!({ "data": { "value": value } }))
			.send()
			.await
			.and_then(|response| response.error_for_status());
		value.zeroize();
		write.map_err(|err| ErrorType::server_error(err))?;
	}

	AppResponse::builder()
		.body(UpdateSecretResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
