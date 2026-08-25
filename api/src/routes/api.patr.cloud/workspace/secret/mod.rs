use axum::{Router, http::StatusCode};
use models::{ErrorType, api::workspace::secret::*, utils::TotalCountHeader};
use time::OffsetDateTime;
use zeroize::Zeroize;

use crate::{prelude::*, utils::openbao::OpenBaoClient};

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState, allowed_client_type: ClientType) -> Router {
	Router::new()
		.mount_auth_endpoint(create_secret, state, allowed_client_type)
		.mount_auth_endpoint(delete_secret, state, allowed_client_type)
		.mount_auth_endpoint(get_secret_info, state, allowed_client_type)
		.mount_auth_endpoint(list_secrets_for_workspace, state, allowed_client_type)
		.mount_auth_endpoint(update_secret, state, allowed_client_type)
		.with_state(state.clone())
}

/// Creates a secret in the workspace: registers the RBAC resource and the secret
/// metadata row in Postgres, and writes the value into OpenBao. The plaintext
/// value never touches Postgres and is zeroized once written.
async fn create_secret(
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
				owner_id,
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
	.await?;

	// Write the value to OpenBao last, so a failure rolls back the DB inserts.
	// Zeroize the plaintext regardless of the outcome.
	let write = OpenBaoClient::new(&state.config.open_bao)
		.write_secret(workspace_id, secret_id, &value)
		.await;
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

/// Deletes a secret: removes the metadata row (rejecting the delete if a
/// deployment still references it), soft-deletes the RBAC resource, and destroys
/// the value in OpenBao.
async fn delete_secret(
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

	query!(
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
	})?;

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

	OpenBaoClient::new(&state.config.open_bao)
		.delete_secret(workspace_id, secret_id)
		.await
		.map_err(|err| ErrorType::server_error(err))?;

	AppResponse::builder()
		.body(DeleteSecretResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}

/// Gets the metadata of a single secret: id, name, created, and last-updated.
/// The value lives in OpenBao and is never read here.
async fn get_secret_info(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: GetSecretInfoPath {
					workspace_id,
					secret_id,
				},
				query: (),
				headers: _,
				body: GetSecretInfoRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		user_data: _,
		state: _,
	}: AuthenticatedAppRequest<'_, GetSecretInfoRequest>,
) -> Result<AppResponse<GetSecretInfoRequest>, ErrorType> {
	trace!("Getting secret info for ID: `{secret_id}`");

	let secret = query!(
		r#"
		SELECT
			secret.id AS "id: Uuid",
			secret.name AS "name: String",
			resource.created AS "created: OffsetDateTime",
			secret.last_updated AS "last_updated: OffsetDateTime"
		FROM
			secret
		JOIN
			resource
		ON
			secret.id = resource.id
		WHERE
			secret.id = $1 AND
			secret.workspace_id = $2 AND
			secret.deleted IS NULL;
		"#,
		secret_id as _,
		workspace_id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	.map(|row| {
		WithId::new(
			row.id,
			Secret {
				name: row.name,
				deployment_id: None,
				created: row.created,
				last_updated: row.last_updated,
			},
		)
	})
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	AppResponse::builder()
		.body(GetSecretInfoResponse { secret })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

/// Lists the secrets in a workspace. Only metadata (id and name) is returned —
/// the value lives in OpenBao and is never read here.
async fn list_secrets_for_workspace(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ListSecretsForWorkspacePath { workspace_id },
				query:
					ListResourceQueryProcessed {
						sort: _,
						search: _,
						count,
						page,
						additional_query: (),
					},
				headers: _,
				body: ListSecretsForWorkspaceRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		user_data: _,
		state: _,
	}: AuthenticatedAppRequest<'_, ListSecretsForWorkspaceRequest>,
) -> Result<AppResponse<ListSecretsForWorkspaceRequest>, ErrorType> {
	trace!("Listing secrets in workspace ID: `{workspace_id}`");

	let mut total_count = 0;
	let secrets = query!(
		r#"
		SELECT
			secret.id AS "id: Uuid",
			secret.name AS "name: String",
			resource.created AS "created: OffsetDateTime",
			secret.last_updated AS "last_updated: OffsetDateTime",
			COUNT(*) OVER() AS "total_count!"
		FROM
			secret
		JOIN
			resource
		ON
			secret.id = resource.id
		WHERE
			secret.workspace_id = $1 AND
			secret.deleted IS NULL
		ORDER BY
			resource.created DESC
		LIMIT $2
		OFFSET $3;
		"#,
		workspace_id as _,
		count as i32,
		(page * count) as i32,
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|row| {
		total_count = row.total_count;
		WithId::new(
			row.id,
			Secret {
				name: row.name,
				deployment_id: None,
				created: row.created,
				last_updated: row.last_updated,
			},
		)
	})
	.collect::<Vec<_>>();

	AppResponse::builder()
		.body(ListSecretsForWorkspaceResponse { secrets })
		.headers(ListSecretsForWorkspaceResponseHeaders {
			total_count: TotalCountHeader(total_count as _),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

/// Updates a secret: renames the metadata row and, when a new value is provided,
/// overwrites the value in OpenBao. An omitted value keeps the existing one. The
/// plaintext value never touches Postgres and is zeroized once written.
async fn update_secret(
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
		let write = OpenBaoClient::new(&state.config.open_bao)
			.write_secret(workspace_id, secret_id, &value)
			.await;
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
