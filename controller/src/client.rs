use std::sync::OnceLock;

use models::{utils::Headers, ApiEndpoint, ApiRequest, ApiResponseBody, AppResponse, ErrorType};
use reqwest::Client;
use serde::{de::DeserializeOwned, Serialize};

static CLIENT: OnceLock<Client> = OnceLock::new();

pub async fn make_request<E>(request: ApiRequest<E>) -> Result<AppResponse<E>, ErrorType>
where
	E: ApiEndpoint,
	E::ResponseBody: DeserializeOwned,
	E::RequestBody: Serialize,
{
	let body = serde_json::to_value(request.body)
		.map_err(|err| err.to_string())
		.map_err(ErrorType::server_error)?;
	let mut reqwest = CLIENT
		.get_or_init(|| Client::new())
		.request(E::METHOD, request.path.to_string())
		.headers(request.headers.to_header_map())
		.query(&request.query);

	if !body.is_null() {
		reqwest = reqwest.json(&body);
	}

	let response = CLIENT
		.get_or_init(|| Client::new())
		.execute(
			reqwest
				.build()
				.map_err(|err| err.to_string())
				.map_err(ErrorType::server_error)?,
		)
		.await
		.map_err(|err| err.to_string())
		.map_err(ErrorType::server_error)?;

	let status_code = response.status();
	let headers = response.headers().clone();

	let body = response
		.json::<ApiResponseBody<E::ResponseBody>>()
		.await
		.map_err(|err| err.to_string())
		.map_err(ErrorType::server_error)?;

	match body {
		ApiResponseBody::Success(data) => Ok(AppResponse::builder()
			.status_code(status_code)
			.headers(
				<E::ResponseHeaders as Headers>::from_header_map(&headers)
					.ok_or_else(|| ErrorType::server_error("Failed to parse response headers"))?,
			)
			.body(data.response)
			.build()),
		ApiResponseBody::Error(body) => Err(body.error),
	}
}
