use axum::http::StatusCode;
use models::api::workspace::service_account::*;
use rand::{RngExt, distr::Alphanumeric};
use sha2::{Digest as _, Sha256};

use crate::{models::permissions, prelude::*};

pub async fn regenerate_service_account_token(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path:
					RegenerateServiceAccountTokenPath {
						workspace_id: _,
						service_account_id,
					},
				query: (),
				headers:
					RegenerateServiceAccountTokenRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: RegenerateServiceAccountTokenRequestProcessed,
			},
		database,
		redis,
		client_ip: _,
		actor_data: _,
		state: _,
	}: AuthenticatedAppRequest<'_, RegenerateServiceAccountTokenRequest>,
) -> Result<AppResponse<RegenerateServiceAccountTokenRequest>, ErrorType> {
	let token = format!(
		"{}{}",
		constants::SERVICE_ACCOUNT_TOKEN_PREFIX,
		rand::rng()
			.sample_iter(Alphanumeric)
			.take(constants::OPAQUE_TOKEN_SECRET_LENGTH)
			.map(char::from)
			.collect::<String>()
	);
	let token_hash = hex::encode(Sha256::digest(&token));

	// `old` is the row as it was before the update (Postgres 18), so this hands
	// back the hash being replaced, whose cache entry has to go.
	let Some(old) = query!(
		r#"
		UPDATE
			service_account
		SET
			token_hash = $1
		WHERE
			id = $2 AND
			deleted IS NULL
		RETURNING
			old.token_hash;
		"#,
		&token_hash,
		service_account_id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	else {
		return Err(ErrorType::ResourceDoesNotExist);
	};

	permissions::mark_token_stale(redis, &service_account_id, &old.token_hash).await?;

	AppResponse::builder()
		.body(RegenerateServiceAccountTokenResponse { token })
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
