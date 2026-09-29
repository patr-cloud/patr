use models::api::user::*;
use rand::{RngExt, distr::Alphanumeric};
use reqwest::StatusCode;
use sha2::{Digest as _, Sha256};

use crate::{models::permissions, prelude::*};

pub async fn regenerate_api_token(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: RegenerateApiTokenPath { token_id },
				query: (),
				headers:
					RegenerateApiTokenRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: RegenerateApiTokenRequestProcessed,
			},
		database,
		redis,
		client_ip: _,
		actor_data,
		state: _,
	}: AuthenticatedAppRequest<'_, RegenerateApiTokenRequest>,
) -> Result<AppResponse<RegenerateApiTokenRequest>, ErrorType> {
	trace!("Regenerating API token: {}", token_id);

	let token = format!(
		"{}{}",
		constants::API_TOKEN_PREFIX,
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
			user_api_token
		SET
			token_hash = $1
		WHERE
			token_id = $2 AND
			user_id = $3
		RETURNING
			old.token_hash;
		"#,
		token_hash,
		token_id as _,
		actor_data.id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	else {
		return Err(ErrorType::ApiTokenDoesNotExist);
	};

	permissions::mark_token_stale(redis, &token_id, &old.token_hash).await?;

	AppResponse::builder()
		.body(RegenerateApiTokenResponse { token })
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
