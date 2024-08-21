use models::api::workspace::database::*;

use crate::prelude::*;

#[server(DeleteDatabaseFn, endpoint = "/infrastructure/database/delete")]
pub async fn delete_database(
	access_token: Option<String>,
	database_id: Option<String>,
	workspace_id: Option<String>,
) -> Result<DeleteDatabaseResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	use constants::USER_AGENT_STRING;

	let access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	let workspace_id = Uuid::parse_str(workspace_id.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	let database_id = Uuid::parse_str(database_id.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	let api_response = make_api_call::<DeleteDatabaseRequest>(
		ApiRequest::builder()
			.path(DeleteDatabasePath {
				database_id,
				workspace_id,
			})
			.query(())
			.headers(DeleteDatabaseRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_static(USER_AGENT_STRING),
			})
			.body(DeleteDatabaseRequest)
			.build(),
	)
	.await;

	api_response
		.map(|res| res.body)
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::InternalServerError))
}