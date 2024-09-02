use models::api::workspace::database::*;

use crate::prelude::*;

#[server(ListDatabaseFn, endpoint = "/infrastructure/database/list")]
pub async fn list_database(
	access_token: Option<String>,
	workspace_id: Option<String>,
) -> Result<ListDatabaseResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	use constants::USER_AGENT_STRING;

	let access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	let workspace_id = Uuid::parse_str(workspace_id.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	let api_response = make_api_call::<ListDatabaseRequest>(
		ApiRequest::builder()
			.path(ListDatabasePath { workspace_id })
			.query(Paginated::default())
			.headers(ListDatabaseRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_static(USER_AGENT_STRING),
			})
			.body(ListDatabaseRequest)
			.build(),
	)
	.await;

	api_response
		.map(|res| res.body)
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::InternalServerError))
}
