use std::str::FromStr;

use axum::{
	RequestExt,
	Router,
	body::Body,
	extract::State,
	http::{Request, StatusCode, header::AUTHORIZATION},
	middleware::{self, Next},
	response::{IntoResponse, Response},
	routing::post,
};
use models::ApiErrorResponse;

use crate::{models::permissions, prelude::*, utils::extractors::ClientIP};

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.route(
			"/loki/api/v1/push",
			post(push_logs).route_layer(middleware::from_fn_with_state(
				state.clone(),
				authenticator,
			)),
		)
		.route(
			"/otlp/v1/logs",
			post(push_otlp_logs).route_layer(middleware::from_fn_with_state(
				state.clone(),
				authenticator,
			)),
		)
		.fallback(unauthorized)
		.with_state(state.clone())
}

#[instrument(skip(req, next))]
async fn authenticator(
	State(mut state): State<AppState>,
	mut req: Request<Body>,
	next: Next,
) -> Response {
	trace!("Ingest route called: {} {}", req.method(), req.uri().path());

	let Some(auth_header) = req
		.headers()
		.get(AUTHORIZATION)
		.and_then(|header| header.to_str().ok())
	else {
		return ApiErrorResponse::error_with_message(
			ErrorType::Unauthorized,
			"Missing Authorization header",
		)
		.into_response();
	};

	let Ok(BearerToken(token)) = BearerToken::from_str(auth_header) else {
		return ApiErrorResponse::error_with_message(
			ErrorType::MalformedAccessToken,
			"Invalid Authorization header format",
		)
		.into_response();
	};

	let token = token.token();

	let (state, redis) = (state.clone(), &mut state.redis);

	let Ok(mut database) = state.database.begin().await else {
		debug!("Failed to begin database transaction");

		return ApiErrorResponse::error_with_message(
			ErrorType::InternalServerError,
			"Unable to begin database transaction",
		)
		.into_response();
	};

	let Ok(ClientIP(client_ip)) = req.extract_parts().await;

	let Ok(_) = permissions::get_user_data_for_token(
		&mut database,
		redis,
		ClientType::ApiToken,
		&state.config,
		client_ip,
		token,
	)
	.await
	else {
		return ApiErrorResponse::error_with_message(
			ErrorType::Unauthorized,
			"Invalid or expired access token",
		)
		.into_response();
	};

	next.run(req).await
}

#[instrument(skip(req))]
async fn push_logs(req: Request<Body>) -> Response {
	proxy_to_loki(req, "/loki/api/v1/push").await
}

#[instrument(skip(req))]
async fn push_otlp_logs(req: Request<Body>) -> Response {
	proxy_to_loki(req, "/otlp/v1/logs").await
}

#[instrument]
async fn unauthorized() -> StatusCode {
	StatusCode::UNAUTHORIZED
}

#[instrument(skip(req))]
async fn proxy_to_loki(req: Request<Body>, path: &'static str) -> Response {
	let Ok(response) = reqwest::Client::new()
		.request(req.method().clone(), format!("http://loki:3100{path}"))
		.headers(req.headers().clone())
		.body(reqwest::Body::wrap_stream(
			req.into_body().into_data_stream(),
		))
		.send()
		.await
		.inspect_err(|err| {
			error!("Error proxying request to loki: {}", err);
		})
	else {
		return Response::builder()
			.status(502)
			.body(Body::from("Bad Gateway"))
			.unwrap();
	};

	let status = response.status();
	let headers = response.headers().clone();
	let body = response.bytes_stream();

	let mut response = Response::builder().status(status);

	for (key, value) in headers.iter() {
		if key != "transfer-encoding" {
			response = response.header(key, value);
		}
	}

	response.body(Body::from_stream(body)).unwrap()
}
