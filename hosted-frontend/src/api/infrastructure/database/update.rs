use std::error::Error;


use crate::prelude::*;

#[server(UpdateDatabaseFn, endpoint = "/infrastructure/database/update")]
pub async fn get_database(
	access_token: Option<String>,
	database_id: Option<String>,
	workspace_id: Option<String>,
	password: String,
) -> Result<(), ServerFnError<ErrorType>> {
	use std::str::FromStr;

	

	let access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	let workspace_id = Uuid::parse_str(workspace_id.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	let database_id = Uuid::parse_str(database_id.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	let api_response = Ok(());

	api_response.map_err(|_: Box<dyn Error>| {
		ServerFnError::WrappedServerError(ErrorType::InternalServerError)
	})
}
