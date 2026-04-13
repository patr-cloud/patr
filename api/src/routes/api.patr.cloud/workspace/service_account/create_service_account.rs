use argon2::{Algorithm, Argon2, PasswordHasher, Version, password_hash::generate_salt};
use axum::http::StatusCode;
use models::api::workspace::service_account::*;

use crate::prelude::*;

pub async fn create_service_account(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: CreateServiceAccountPath { workspace_id },
				query: (),
				headers:
					CreateServiceAccountRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body:
					CreateServiceAccountRequestProcessed {
						name,
						description,
						roles,
					},
			},
		database,
		redis: _,
		client_ip: _,
		user_data: _,
		state,
	}: AuthenticatedAppRequest<'_, CreateServiceAccountRequest>,
) -> Result<AppResponse<CreateServiceAccountRequest>, ErrorType> {
	info!("Creating service account with name: `{name}`");

	// Generate token
	let refresh_token = Uuid::new_v4();
	let token_hash = Argon2::new_with_secret(
		state.config.password_pepper.as_bytes(),
		Algorithm::Argon2id,
		Version::V0x13,
		constants::HASHING_PARAMS,
	)
	.map_err(ErrorType::server_error)?
	.hash_password_with_salt(refresh_token.as_bytes(), &generate_salt())
	.map_err(ErrorType::server_error)?
	.to_string();

	// Create resource
	let id = query!(
		r#"
		INSERT INTO
			resource(
				id,
				resource_type_id,
				owner_id,
				created
			)
		VALUES
			(
				GENERATE_RESOURCE_ID(),
				(SELECT id FROM resource_type WHERE name = 'serviceAccount'),
				$1,
				NOW()
			)
		RETURNING id AS "id: Uuid";
		"#,
		workspace_id as _,
	)
	.fetch_one(&mut **database)
	.await
	.map_err(|e| match e {
		sqlx::Error::Database(dbe) if dbe.is_unique_violation() => ErrorType::ResourceAlreadyExists,
		other => other.into(),
	})?
	.id;

	// Create service account
	query!(
		r#"
		INSERT INTO
			service_account(
				id,
				name,
				workspace_id,
				created,
				description,
				token_hash
			)
		VALUES
			($1, $2, $3, NOW(), $4, $5);
		"#,
		id as _,
		name.as_ref(),
		workspace_id as _,
		description.as_deref(),
		&token_hash,
	)
	.execute(&mut **database)
	.await
	.map_err(|e| match e {
		sqlx::Error::Database(dbe) if dbe.is_unique_violation() => ErrorType::ResourceAlreadyExists,
		other => other.into(),
	})?;

	// Assign roles
	for role_id in &roles {
		query!(
			r#"
			INSERT INTO
				service_account_role(
					service_account_id,
					workspace_id,
					role_id
				)
			VALUES
				($1, $2, $3);
			"#,
			id as _,
			workspace_id as _,
			role_id as _,
		)
		.execute(&mut **database)
		.await?;
	}

	let token = format!("patrv1.{}.{}", refresh_token, id);

	AppResponse::builder()
		.body(CreateServiceAccountResponse {
			id: WithId::from(id),
			token,
		})
		.headers(())
		.status_code(StatusCode::CREATED)
		.build()
		.into_result()
}
