use models::api::workspace::runner::*;

use crate::prelude::*;

/// Server function to delete a runner
#[server(DeleteRunnerFn, endpoint = "/infrastructure/runner/delete")]
pub async fn delete_runner(
	access_token: Option<String>,
	workspace_id: Option<Uuid>,
	runner_id: Uuid,
) -> Result<DeleteRunnerResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	let access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	let workspace_id = workspace_id
		.ok_or_else(|| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	make_api_call::<DeleteRunnerRequest>(
		ApiRequest::builder()
			.path(DeleteRunnerPath {
				workspace_id,
				runner_id,
			})
			.query(())
			.headers(DeleteRunnerRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_static("todo"),
			})
			.body(DeleteRunnerRequest)
			.build(),
	)
	.await
	.map(|res| {
		leptos_axum::redirect("/runners");
		res.body
	})
	.map_err(ServerFnError::WrappedServerError)
}
