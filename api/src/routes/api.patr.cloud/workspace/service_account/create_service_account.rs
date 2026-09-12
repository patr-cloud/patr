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
						role_bindings,
					},
			},
		database,
		redis: _,
		client_ip: _,
		user_data,
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
				workspace_id,
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

	// The same id registers the account in all three supertypes: it is a
	// resource above, an actor_client (so it can authenticate and be named in
	// an audit entry) and a workspace_actor (so role bindings can hang off
	// it).
	query!(
		r#"
		INSERT INTO
			actor_client(
				id,
				actor_client_type
			)
		VALUES
			($1, 'service_account');
		"#,
		id as _,
	)
	.execute(&mut **database)
	.await?;

	query!(
		r#"
		INSERT INTO
			workspace_actor(
				id,
				workspace_id,
				actor_type
			)
		VALUES
			($1, $2, 'service_account');
		"#,
		id as _,
		workspace_id as _,
	)
	.execute(&mut **database)
	.await?;

	// Create service account
	query!(
		r#"
		INSERT INTO
			service_account(
				id,
				workspace_id,
				name,
				description,
				token_hash,
				created
			)
		VALUES
			($1, $2, $3, $4, $5, NOW());
		"#,
		id as _,
		workspace_id as _,
		name.as_ref(),
		description.as_deref(),
		&token_hash,
	)
	.execute(&mut **database)
	.await
	.map_err(|e| match e {
		sqlx::Error::Database(dbe) if dbe.is_unique_violation() => ErrorType::ResourceAlreadyExists,
		other => other.into(),
	})?;

	// Every grant is a binding: a role plus the one resource it applies at.
	// The scope FK pivots on workspace_id, so a role or a resource from
	// another workspace is rejected by the database rather than here.
	for grant in &role_bindings {
		query!(
			r#"
			INSERT INTO
				role_binding(
					id,
					workspace_id,
					actor_id,
					role_id,
					scope_id,
					created,
					created_by
				)
			VALUES
				(GEN_RANDOM_UUID(), $1, $2, $3, $4, NOW(), $5);
			"#,
			workspace_id as _,
			id as _,
			grant.role_id as _,
			grant.resource_id as _,
			user_data.id as _,
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
