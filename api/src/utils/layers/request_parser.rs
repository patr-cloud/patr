use std::{
	future::Future,
	marker::PhantomData,
	task::{Context, Poll},
};

use axum::{
	body::HttpBody,
	extract::{FromRequest, FromRequestParts, Path, Query},
	http::Request,
	response::{IntoResponse, Response},
	BoxError,
	Json,
};
use models::{
	utils::{ApiErrorResponse, ApiRequest, Headers, IntoAxumResponse},
	ErrorType,
};
use sea_orm::TransactionTrait;
use tower::{Layer, Service};

use crate::{app::AppResponse, prelude::*};

#[derive(Clone, Debug)]
pub struct RequestParserLayer<E>
where
	E: ApiEndpoint,
{
	phantom: PhantomData<E>,
	state: AppState,
}

impl<E> RequestParserLayer<E>
where
	E: ApiEndpoint,
{
	pub fn with_state(state: AppState) -> Self {
		Self {
			phantom: PhantomData,
			state,
		}
	}
}

impl<S, E> Layer<S> for RequestParserLayer<E>
where
	S: Service<AppRequest<E>>,
	E: ApiEndpoint,
{
	type Service = RequestParser<S, E>;

	fn layer(&self, inner: S) -> Self::Service {
		RequestParser {
			inner,
			state: self.state.clone(),
			phantom: PhantomData,
		}
	}
}

#[derive(Clone, Debug)]
pub struct RequestParser<S, E>
where
	S: Service<AppRequest<E>>,
	E: ApiEndpoint,
{
	inner: S,
	state: AppState,
	phantom: PhantomData<E>,
}

impl<B, S, E> Service<Request<B>> for RequestParser<S, E>
where
	B: HttpBody + Send + 'static,
	B::Data: Send,
	B::Error: Into<BoxError>,
	E: ApiEndpoint,
	S: Service<AppRequest<E>> + Clone,
	S::Response: Into<AppResponse<E>>,
	S::Error: Into<ErrorType>,
{
	type Response = Response;
	type Error = Response;
	type Future = impl Future<Output = Result<Self::Response, Self::Error>>;

	fn poll_ready(
		&mut self,
		cx: &mut Context<'_>,
	) -> Poll<Result<(), Self::Error>> {
		self.inner
			.poll_ready(cx)
			.map_err(|_| IntoResponse::into_response(()))
	}

	#[instrument(skip(self, req))]
	fn call(&mut self, req: Request<B>) -> Self::Future {
		let state = self.state.clone();
		let mut inner = self.inner.clone();
		async move {
			let (mut parts, body) = req.into_parts();
			let Ok(Path(path)) =
				FromRequestParts::from_request_parts(&mut parts, &state).await
			else {
				debug!("Failed to parse path: {}", parts.uri.path());
				return Err(ApiErrorResponse::error_with_message(
					ErrorType::WrongParameters,
					"Invalid Request URL",
				)
				.into_response());
			};

			let Ok(Query(query)) =
				FromRequestParts::from_request_parts(&mut parts, &state).await
			else {
				debug!("Failed to parse query: {:?}", parts.uri.query());
				return Err(ApiErrorResponse::error_with_message(
					ErrorType::WrongParameters,
					"Invalid Query Parameters",
				)
				.into_response());
			};

			let Some(headers) =
				<E::RequestHeaders as Headers>::from_header_map(&parts.headers)
			else {
				debug!("Failed to parse headers");
				return Err(ApiErrorResponse::error_with_message(
					ErrorType::WrongParameters,
					"Invalid Headers",
				)
				.into_response());
			};

			let req = Request::from_parts(parts, body);
			let Ok(Json(body)) = FromRequest::from_request(req, &state).await
			else {
				debug!("Failed to parse body");
				return Err(ApiErrorResponse::error_with_message(
					ErrorType::WrongParameters,
					"Invalid body",
				)
				.into_response());
			};

			let Ok(database) = state.database.begin().await else {
				debug!("Failed to begin database transaction");
				return Err(ApiErrorResponse::internal_error(
					"unable to begin database transaction",
				)
				.into_response());
			};

			let redis = state.redis.create_transaction();

			let req = AppRequest {
				request: ApiRequest {
					path,
					query,
					headers,
					body,
				},
				database,
				redis,
				config: state.config.clone(),
			};

			let response = inner
				.call(req)
				.await
				.map(|response| {
					let AppResponse::<E> {
						status_code,
						headers,
						body,
					} = response.into();
					(
						status_code,
						headers.to_header_map(),
						body.into_axum_response(),
					)
						.into_response()
				})
				.map_err(|err| todo!());

			response
		}
	}
}
