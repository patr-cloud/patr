use std::{thread, time};

use models::api::workspace::deployment::*;
use server_fn::codec::FromRes;

use crate::prelude::*;

/// List Deployments
#[server(ListDeploymentFn, endpoint = "/infrastructure/deployment/list")]
pub async fn list_deployments(
	access_token: Option<String>,
	workspace_id: Uuid,
	page: Option<usize>,
	count: Option<usize>,
) -> Result<(usize, ListDeploymentResponse), ServerFnError<ErrorType>> {
	use std::str::FromStr;

	thread::sleep(time::Duration::from_secs(2));
	let access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	make_api_call::<ListDeploymentRequest>(
		ApiRequest::builder()
			.path(ListDeploymentPath { workspace_id })
			.query(Paginated {
				data: (),
				page: page.unwrap_or(0),
				count: count.unwrap_or(10),
			})
			.headers(ListDeploymentRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_static("todo"),
			})
			.body(ListDeploymentRequest)
			.build(),
	)
	.await
	.map(|res| (res.headers.total_count.0, res.body))
	.map_err(ServerFnError::WrappedServerError)
}
