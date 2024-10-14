use models::api::workspace::database::*;

use crate::prelude::*;

#[server(
	ListMachineTypesFn,
	endpoint = "/infrastructure/database/machine-types"
)]
pub async fn list_database_machine_types(
	access_token: Option<String>,
) -> Result<ListAllDatabaseMachineTypeResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	let access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	make_api_call::<ListAllDatabaseMachineTypeRequest>(
		ApiRequest::builder()
			.path(ListAllDatabaseMachineTypePath)
			.query(())
			.headers(ListAllDatabaseMachineTypeRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_static("todo"),
			})
			.body(ListAllDatabaseMachineTypeRequest)
			.build(),
	)
	.await
	.map(|res| res.body)
	.map_err(|_| ServerFnError::WrappedServerError(ErrorType::InternalServerError))
}
