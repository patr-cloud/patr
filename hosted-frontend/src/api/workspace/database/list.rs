use models::api::workspace::database::*;

use crate::prelude::*;

#[server(ListDatabaseFn, endpoint = "/infrastructure/database/list")]
pub async fn list_database(
	access_token: Option<String>,
	workspace_id: Option<Uuid>,
) -> Result<ListDatabaseResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	let access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	let workspace_id = workspace_id
		.ok_or_else(|| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	make_api_call::<ListDatabaseRequest>(
		ApiRequest::builder()
			.path(ListDatabasePath { workspace_id })
			.query(Paginated::default())
			.headers(ListDatabaseRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_static("todo"),
			})
			.body(ListDatabaseRequest)
			.build(),
	)
	.await
	.map(|res| res.body)
	.map_err(|_| ServerFnError::WrappedServerError(ErrorType::InternalServerError))
}
