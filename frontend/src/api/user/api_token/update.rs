use std::collections::BTreeMap;

use leptos::server_fn::codec::Json;
use models::{api::user::*, rbac::WorkspacePermission};

use crate::prelude::*;

#[server(UpdateApiTokenFn, endpoint = "/user/api-token/update", input = Json)]
pub async fn update_api_token(
	access_token: Option<String>,
	token_id: Uuid,
	update_token_body: UpdateApiTokenRequest,
) -> Result<UpdateApiTokenResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	let access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	make_api_call::<UpdateApiTokenRequest>(
		ApiRequest::builder()
			.path(UpdateApiTokenPath { token_id })
			.query(())
			.headers(UpdateApiTokenRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_str("hyper/0.12.2").unwrap(),
			})
			.body(update_token_body)
			.build(),
	)
	.await
	.map(|res| res.body)
	.map_err(ServerFnError::WrappedServerError)
}
