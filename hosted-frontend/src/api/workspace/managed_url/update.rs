use std::str::FromStr;

use models::api::workspace::managed_url::*;

use crate::prelude::*;

#[server(UpdateManagedUrlFn, endpoint = "/domain-config/managed-url/update")]
pub async fn update_managed_url(
	workspace_id: Option<String>,
	access_token: Option<String>,
	path: String,
	managed_url_id: String,
	url_type: String,
	url: String,
	port: u16,
	http_only: bool,
	permanent_redirect: bool,
) -> Result<UpdateManagedURLResponse, ServerFnError<ErrorType>> {
	use constants::USER_AGENT_STRING;

	let access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	let workspace_id = Uuid::parse_str(workspace_id.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	let managed_url_id = Uuid::parse_str(managed_url_id.as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	logging::log!("error here");
	let managed_url_type = get_managed_url_type(url_type, url, port, http_only, permanent_redirect)
		.ok_or(ServerFnError::WrappedServerError(
			ErrorType::WrongParameters,
		))?;

	logging::log!("error here");
	let req_body = UpdateManagedURLRequest {
		path,
		url_type: ManagedUrlType::ProxyUrl {
			url: "vicara.co".to_string(),
			http_only: false,
		},
	};

	let api_response: Result<_, _> = make_api_call::<UpdateManagedURLRequest>(
		ApiRequest::builder()
			.path(UpdateManagedURLPath {
				workspace_id,
				managed_url_id,
			})
			.query(())
			.headers(UpdateManagedURLRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_static(USER_AGENT_STRING),
			})
			.body(req_body)
			.build(),
	)
	.await;

	api_response
		.map(|res| res.body)
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::InternalServerError))
}
