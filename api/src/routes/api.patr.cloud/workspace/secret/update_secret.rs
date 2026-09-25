use axum::http::StatusCode;
use models::api::workspace::{runner::StreamRunnerDataForWorkspaceServerMsg, secret::*};
use rustis::commands::PubSubCommands;
use time::OffsetDateTime;
use zeroize::Zeroize;

use crate::prelude::*;

/// Updates a secret: renames the metadata row and, when a new value is provided,
/// overwrites the value in OpenBao and tells the runners using it. An omitted
/// value keeps the existing one. `last_updated` tracks the value, so a rename
/// alone leaves it (and the deployments using the secret) alone. The plaintext
/// value never touches Postgres and is zeroized once written.
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
		redis,
		client_ip: _,
		user_data: _,
		state,
	}: AuthenticatedAppRequest<'_, UpdateSecretRequest>,
) -> Result<AppResponse<UpdateSecretRequest>, ErrorType> {
	trace!("Updating secret ID: `{secret_id}`");

	let last_updated = query!(
		r#"
		UPDATE
			secret
		SET
			name = $1,
			last_updated = CASE WHEN $2 THEN NOW() ELSE last_updated END
		WHERE
			id = $3
		RETURNING
			last_updated AS "last_updated: OffsetDateTime";
		"#,
		name as _,
		value.is_some(),
		secret_id as _,
	)
	.fetch_one(&mut **database)
	.await
	.map_err(|e| match e {
		sqlx::Error::Database(dbe) if dbe.is_unique_violation() => ErrorType::ResourceAlreadyExists,
		sqlx::Error::RowNotFound => ErrorType::ResourceDoesNotExist,
		other => other.into(),
	})?
	.last_updated;

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

		// TODO Temporary workaround until audit logs and triggers are implemented
		let runners = query!(
			r#"
			SELECT DISTINCT
				deployment.runner AS "runner: Uuid"
			FROM
				deployment_environment_variable
			INNER JOIN
				deployment
			ON
				deployment.id = deployment_environment_variable.deployment_id
			WHERE
				deployment_environment_variable.secret_id = $1 AND
				deployment.deleted IS NULL;
			"#,
			secret_id as _,
		)
		.fetch_all(&mut **database)
		.await?;

		for row in runners {
			redis
				.publish(
					format!("{}/runner/{}/stream", workspace_id, row.runner),
					serde_json::to_string(&StreamRunnerDataForWorkspaceServerMsg::SecretUpdated {
						id: secret_id,
						last_updated,
					})
					.unwrap(),
				)
				.await?;
		}
	}

	AppResponse::builder()
		.body(UpdateSecretResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
