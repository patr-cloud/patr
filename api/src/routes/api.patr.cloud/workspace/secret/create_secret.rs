use axum::http::StatusCode;
use models::api::workspace::secret::*;
use time::OffsetDateTime;
use zeroize::Zeroize;

use crate::prelude::*;

/// Creates a secret in the workspace: registers the RBAC resource and the secret
/// metadata row in Postgres, and writes the value into OpenBao. The plaintext
/// value never touches Postgres and is zeroized once written.
pub async fn create_secret(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: CreateSecretPath { workspace_id },
				query: (),
				headers:
					CreateSecretRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: CreateSecretRequestProcessed { name, mut value },
			},
		database,
		redis: _,
		client_ip: _,
		user_data: _,
		state,
	}: AuthenticatedAppRequest<'_, CreateSecretRequest>,
) -> Result<AppResponse<CreateSecretRequest>, ErrorType> {
	trace!("Creating secret with name: {name}");

	let now = OffsetDateTime::now_utc();
	let secret_id = query!(
		r#"
		INSERT INTO
			resource(
				id,
				resource_type_id,
				workspace_id,
				created,
				deleted
			)
		VALUES
			(
				GENERATE_RESOURCE_ID(),
				(SELECT id FROM resource_type WHERE name = 'secret'),
				$1,
				$2,
				NULL
			)
		RETURNING id AS "id: Uuid";
		"#,
		workspace_id as _,
		now as _,
	)
	.fetch_one(&mut **database)
	.await?
	.id;

	query!(
		r#"
		INSERT INTO
			secret(
				id,
				name,
				workspace_id,
				last_updated,
				deleted
			)
		VALUES
			(
				$1,
				$2,
				$3,
				$4,
				NULL
			);
		"#,
		secret_id as _,
		name as _,
		workspace_id as _,
		now as _,
	)
	.execute(&mut **database)
	.await
	.map_err(|e| match e {
		sqlx::Error::Database(dbe) if dbe.is_unique_violation() => ErrorType::ResourceAlreadyExists,
		other => other.into(),
	})?;

	// Write the value to OpenBao last, so a failure rolls back the DB inserts.
	// Zeroize the plaintext regardless of the outcome.
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

	AppResponse::builder()
		.body(CreateSecretResponse {
			id: WithId::from(secret_id),
		})
		.headers(())
		.status_code(StatusCode::CREATED)
		.build()
		.into_result()
}
