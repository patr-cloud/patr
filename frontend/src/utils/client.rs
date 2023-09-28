use std::{future::Future, marker::PhantomData, pin::Pin, str::FromStr, sync::OnceLock};

use axum_extra::routing::TypedPath;
use leptos::{server_fn::ServerFn, Scope, ServerFnError};
use models::{
	prelude::*,
	utils::{constants, False, Headers},
	ApiErrorResponse,
	ApiErrorResponseBody,
	ApiResponseBody,
	ApiSuccessResponseBody,
};
use reqwest::Client;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use url::Url;

static REQUEST_CLIENT: OnceLock<Client> = OnceLock::new();

/// Makes a request to the API. Requires an ApiRequest object for a specific
/// endpoint, and returns the response corresponding to that endpoint
/// TODO implement automatically refreshing tokens
pub async fn make_request<E>(
	ApiRequest {
		path,
		query,
		headers,
		body,
	}: ApiRequest<E>,
) -> Result<ApiSuccessResponse<E>, ApiErrorResponse>
where
	E: ApiEndpoint,
	E::ResponseBody: DeserializeOwned + Serialize,
	E::RequestBody: DeserializeOwned + Serialize,
{
	let body = serde_json::to_value(&body).unwrap();
	let builder = REQUEST_CLIENT
		.get_or_init(initialize_client)
		.request(
			E::METHOD,
			Url::from_str(constants::API_BASE_URL)
				.unwrap()
				.join(path.to_string().as_str())
				.unwrap(),
		)
		.query(&query)
		.headers(headers.to_header_map());
	let response = if body.is_null() {
		builder
	} else {
		builder.json(&body)
	}
	.send()
	.await;

	let response = match response {
		Ok(response) => response,
		Err(error) => {
			return Err(ApiErrorResponse {
				status_code: reqwest::StatusCode::INTERNAL_SERVER_ERROR,
				body: ApiErrorResponseBody {
					success: False,
					error: ErrorType::server_error(error.to_string()),
					message: error.to_string(),
				},
			});
		}
	};

	let status_code = response.status();
	let Some(headers) = E::ResponseHeaders::from_header_map(response.headers()) else {
		return Err(ApiErrorResponse {
			status_code: reqwest::StatusCode::INTERNAL_SERVER_ERROR,
			body: ApiErrorResponseBody {
				success: False,
				error: ErrorType::server_error("invalid headers"),
				message: "invalid headers".to_string(),
			},
		});
	};

	match response.json::<ApiResponseBody<E::ResponseBody>>().await {
		Ok(ApiResponseBody::Success(ApiSuccessResponseBody {
			success: _,
			response: body,
		})) => Ok(ApiSuccessResponse {
			status_code,
			headers,
			body,
		}),
		Ok(ApiResponseBody::Error(error)) => Err(ApiErrorResponse {
			status_code,
			body: error,
		}),
		Err(error) => {
			tracing::error!("{}", error.to_string());
			Err(ApiErrorResponse {
				status_code: reqwest::StatusCode::INTERNAL_SERVER_ERROR,
				body: ApiErrorResponseBody {
					success: False,
					error: ErrorType::server_error(error.to_string()),
					message: error.to_string(),
				},
			})
		}
	}
}

fn initialize_client() -> Client {
	Client::builder()
		.build()
		.expect("failed to initialize client")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ServerFnRequest<E>
where
	E: ApiEndpoint,
{
	#[serde(flatten)]
	data: E::RequestBody,
	#[serde(skip)]
	phantom: PhantomData<E>,
}

impl<E> ServerFn<Scope> for ServerFnRequest<E>
where
	E: ApiEndpoint,
	E::RequestBody: DeserializeOwned + Serialize,
{
	type Output = Value;

	fn prefix() -> &'static str {
		"/api"
	}

	fn url() -> &'static str {
		<E::RequestPath as TypedPath>::PATH
	}

	fn encoding() -> leptos::server_fn::Encoding {
		leptos::server_fn::Encoding::Url
	}

	#[cfg(target_arch = "wasm32")]
	fn call_fn_client(
		self,
		_cx: Scope,
	) -> Pin<Box<dyn Future<Output = Result<Self::Output, ServerFnError>>>> {
		Box::pin(async move {
			leptos::server_fn::call_server_fn(
				&{ Self::prefix().to_string() + "/" + Self::url() },
				self,
				leptos::server_fn::Encoding::Url,
			)
			.await
		})
	}

	#[cfg(not(target_arch = "wasm32"))]
	fn call_fn(
		self,
		_cx: Scope,
	) -> Pin<Box<dyn Future<Output = Result<Self::Output, ServerFnError>>>> {
		Box::pin(async move {
			println!("Server fn {} called", Self::url());
			Ok(Value::default())
		})
	}
}
