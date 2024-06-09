use std::{collections::BTreeMap, str::FromStr};

use models::api::user::CreateApiTokenResponse;

use crate::prelude::*;

#[server(CreateApiTokenFn, endpoint = "/user/api-token")]
pub async fn create_api_token(
	access_token: Option<String>,
	token_name: String,
) -> Result<CreateApiTokenResponse, ServerFnError<ErrorType>> {
	use models::{
		api::user::{
			CreateApiTokenPath,
			CreateApiTokenRequest,
			CreateApiTokenRequestHeaders,
			UserApiToken,
		},
		rbac::WorkspacePermission,
	};
	use time::OffsetDateTime;

	let access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	let token = UserApiToken {
		name: token_name,
		allowed_ips: None,
		created: OffsetDateTime::now_utc(),
		token_exp: None,
		token_nbf: None,
		permissions: BTreeMap::<Uuid, WorkspacePermission>::new(),
	};

	let api_response = make_api_call::<CreateApiTokenRequest>(
		ApiRequest::builder()
			.path(CreateApiTokenPath)
			.query(())
			.headers(CreateApiTokenRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_static("hyper/0.12.2"),
			})
			.body(CreateApiTokenRequest { token })
			.build(),
	)
	.await;

	api_response
		.map(|res| res.body)
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::InternalServerError))
}
