use axum::http::StatusCode;
use models::api::workspace::secret::*;

use crate::prelude::*;

/// Deletes a secret: removes the metadata row (rejecting the delete if a
/// deployment still references it), soft-deletes the RBAC resource, and destroys
/// the value in OpenBao.
pub async fn delete_secret(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: DeleteSecretPath {
					workspace_id,
					secret_id,
				},
				query: (),
				headers: _,
				body: DeleteSecretRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		user_data: _,
		state,
	}: AuthenticatedAppRequest<'_, DeleteSecretRequest>,
) -> Result<AppResponse<DeleteSecretRequest>, ErrorType> {
	trace!("Deleting secret ID: `{secret_id}`");

	let rows_deleted = query!(
		r#"
		DELETE FROM
			secret
		WHERE
			id = $1;
		"#,
		secret_id as _,
	)
	.execute(&mut **database)
	.await
	.map_err(|err| match err {
		sqlx::Error::Database(dbe) if dbe.is_foreign_key_violation() => ErrorType::ResourceInUse,
		err => ErrorType::server_error(err),
	})?
	.rows_affected();

	if rows_deleted == 0 {
		return Err(ErrorType::ResourceDoesNotExist);
	}

	query!(
		r#"
		UPDATE
			resource
		SET
			deleted = NOW()
		WHERE
			id = $1;
		"#,
		secret_id as _,
	)
	.execute(&mut **database)
	.await?;

	// Deleting the metadata destroys every version of the value.
	reqwest::Client::new()
		.delete(format!(
			"{}/v1/secret/metadata/{}/{}",
			state.config.open_bao.endpoint.trim_end_matches('/'),
			workspace_id,
			secret_id
		))
		.header("X-Vault-Token", &state.config.open_bao.token)
		.send()
		.await
		.and_then(|response| response.error_for_status())
		.map_err(|err| ErrorType::server_error(err))?;

	AppResponse::builder()
		.body(DeleteSecretResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
