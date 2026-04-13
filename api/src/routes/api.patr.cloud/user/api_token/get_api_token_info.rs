use models::api::user::*;
use reqwest::StatusCode;

use crate::{
	models::permissions::{IdentityTokenType, get_permissions_for_identity},
	prelude::*,
};

pub async fn get_api_token_info(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: GetApiTokenInfoPath { token_id },
				query: (),
				headers:
					GetApiTokenInfoRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: GetApiTokenInfoRequestProcessed,
			},
		database,
		redis,
		client_ip: _,
		user_data,
		state: _,
	}: AuthenticatedAppRequest<'_, GetApiTokenInfoRequest>,
) -> Result<AppResponse<GetApiTokenInfoRequest>, ErrorType> {
	trace!("Getting info for API token: {}", token_id);

	let mut token = query!(
		r#"
		SELECT
			token_id,
			name,
			token_nbf,
			token_exp,
			allowed_ips,
			created
		FROM
			user_api_token
		WHERE
			token_id = $1 AND
			user_id = $2 AND
			revoked IS NULL;
		"#,
		token_id as _,
		user_data.id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::ApiTokenDoesNotExist)
	.map(|row| {
		WithId::new(
			row.token_id,
			UserApiToken {
				name: row.name,
				permissions: Default::default(),
				token_nbf: row.token_nbf,
				token_exp: row.token_exp,
				allowed_ips: row.allowed_ips,
				created: row.created,
			},
		)
	})?;

	trace!("Basic token info fetched");

	// Route the read through the same cache/intersect/write-back path the auth
	// layer uses so the UI shows the token's effective permissions — narrowed
	// by any user-side role revocations since the token was minted.
	token.data.permissions = get_permissions_for_identity(
		&mut **database,
		redis,
		&token_id,
		&user_data.id.into(),
		IdentityTokenType::ApiToken,
	)
	.await?;

	AppResponse::builder()
		.body(GetApiTokenInfoResponse { token })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
