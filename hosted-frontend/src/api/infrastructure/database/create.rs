use models::api::workspace::database::*;

use crate::prelude::*;

#[server(CreateDatabaseFn, endpoint = "/infrastructure/database/create")]
pub async fn create_database(
	name: String,
	num_nodes: u16,
	engine: DatabaseEngine,
	access_token: Option<String>,
	workspace_id: Option<String>,
	machine_type: String,
	version: String,
	runner_id: String,
) -> Result<CreateDatabaseResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	use constants::USER_AGENT_STRING;

	let access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	let workspace_id = Uuid::parse_str(workspace_id.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	let runner_id = Uuid::parse_str(runner_id.as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	let machine_type = Uuid::parse_str(machine_type.as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	let req_body = CreateDatabaseRequest {
		name,
		engine,
		num_node: num_nodes,
		database_plan_id: machine_type,
		region: runner_id,
		version,
	};

	let api_response = make_api_call::<CreateDatabaseRequest>(
		ApiRequest::builder()
			.path(CreateDatabasePath { workspace_id })
			.query(())
			.headers(CreateDatabaseRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_static(USER_AGENT_STRING),
			})
			.body(req_body)
			.build(),
	)
	.await;

	if api_response.is_ok() {
		leptos_axum::redirect("/database");
	}

	api_response
		.map(|res| res.body)
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::InternalServerError))
}
