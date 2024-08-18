use std::{fmt::Debug, future::Future, marker::PhantomData};

use axum::body::Body;
use cookie::Cookie;
use http::{header, Request, Response, StatusCode};
use leptos::server_fn::redirect;
use models::ApiEndpoint;
use preprocess::Preprocessable;
use tower::{Layer, Service};

use crate::LoggedOutRoute;

pub struct AuthenticationLayer<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	endpoint: PhantomData<E>,
}

impl<S, E> Layer<S> for AuthenticationLayer<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	type Service = AuthenticationService<S, E>;

	fn layer(&self, inner: S) -> Self::Service {
		AuthenticationService {
			inner,
			endpoint: PhantomData,
		}
	}
}

pub struct AuthenticationService<S, E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	inner: S,
	endpoint: PhantomData<E>,
}

impl<S, E> Service<Request<Body>> for AuthenticationService<S, E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
	S: Service<Request<Body>, Response = Response<Body>> + Clone,
	<S as Service<Request<Body>>>::Error: Debug,
{
	type Error = S::Error;
	type Response = S::Response;

	type Future = impl Future<Output = Result<Self::Response, Self::Error>>;

	fn call(&mut self, req: Request<Body>) -> Self::Future
	where
		<E::RequestBody as Preprocessable>::Processed: Send,
	{
		let mut inner = self.inner.clone();
		async move {
			let (parts, body) = req.into_parts();

			if let Ok(cookie) = Cookie::parse_encoded(
				parts
					.headers
					.get(http::header::COOKIE)
					.and_then(|v| v.to_str().ok())
					.unwrap_or_default(),
			) {
				// TODO parse cookie and check if it's valid
				let req = Request::from_parts(parts, body);
				Ok(inner.call(req).await.unwrap())
			} else {
				// insert the Location header in any case
				let res = Response::builder().header(
					header::LOCATION,
					header::HeaderValue::from_str(LoggedOutRoute::Login.to_string().as_str())
						.expect("Failed to create HeaderValue"),
				);

				let accepts_html = parts
					.headers
					.get(header::ACCEPT)
					.and_then(|v| v.to_str().ok())
					.map(|v| v.contains("text/html"))
					.unwrap_or(false);
				Ok(if accepts_html {
					// if the request accepts text/html, it's a plain form request and needs
					// to have the 302 code set
					res.status(StatusCode::FOUND)
				} else {
					// otherwise, we sent it from the server fn client and actually don't want
					// to set a real redirect, as this will break the ability to return data
					// instead, set the REDIRECT_HEADER to indicate that the client should redirect
					res.header(
						header::HeaderName::from_static(redirect::REDIRECT_HEADER),
						header::HeaderValue::from_str("").unwrap(),
					)
				}
				.body(Body::empty())
				.unwrap())
			}
		}
	}

	fn poll_ready(
		&mut self,
		cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Result<(), Self::Error>> {
		self.inner.poll_ready(cx)
	}
}
