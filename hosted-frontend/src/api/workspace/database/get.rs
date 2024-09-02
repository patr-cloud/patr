use models::api::workspace::database::*;

use crate::prelude::*;

#[server(GetDatabaseFn, endpoint = "/infrastructure/database/get")]
pub async fn get_database(
	access_token: Option<String>,
	database_id: Option<String>,
	workspace_id: Option<Uuid>,
) -> Result<GetDatabaseResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	let access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	let workspace_id = Uuid::parse_str(&workspace_id.unwrap().to_string())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	let database_id = Uuid::parse_str(database_id.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	make_api_call::<GetDatabaseRequest>(
		ApiRequest::builder()
			.path(GetDatabasePath {
				database_id,
				workspace_id,
			})
			.query(())
			.headers(GetDatabaseRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_static("todo"),
			})
			.body(GetDatabaseRequest)
			.build(),
	)
	.await
	.map(|res| res.body)
	.map_err(ServerFnError::WrappedServerError)
}
