use models::api::workspace::database::*;

use crate::prelude::*;

#[server(
	ListMachineTypesFn,
	endpoint = "/infrastructure/database/machine-types"
)]
pub async fn list_database(
	access_token: Option<String>,
) -> Result<ListAllDatabaseMachineTypeResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	use constants::USER_AGENT_STRING;

	let access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	let api_response = make_api_call::<ListAllDatabaseMachineTypeRequest>(
		ApiRequest::builder()
			.path(ListAllDatabaseMachineTypePath)
			.query(())
			.headers(ListAllDatabaseMachineTypeRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_static(USER_AGENT_STRING),
			})
			.body(ListAllDatabaseMachineTypeRequest)
			.build(),
	)
	.await;

	api_response
		.map(|res| res.body)
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::InternalServerError))
}
