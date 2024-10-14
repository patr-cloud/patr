use std::{fmt::Debug, future::Future, marker::PhantomData, pin::Pin};

use axum::{
	body::Body,
	http::{Request, Response},
};
use cookie::Cookie;
use models::ApiEndpoint;
use preprocess::Preprocessable;
use tower::{Layer, Service};

use crate::LoggedOutRoute;

/// Middleware to check if the user is authenticated. If not, it will redirect
/// to the login page with the current path as a query parameter.
pub struct AuthenticationLayer<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	endpoint: PhantomData<E>,
}

impl<E> AuthenticationLayer<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// Create a new instance of the authentication layer.
	pub const fn new() -> Self {
		Self {
			endpoint: PhantomData,
		}
	}
}

impl<E> Clone for AuthenticationLayer<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	fn clone(&self) -> Self {
		Self::new()
	}
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

/// Middleware to check if the user is authenticated. If not, it will redirect
/// to the login page with the current path as a query parameter.
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
	S: Service<Request<Body>, Response = Response<Body>>,
	<S as Service<Request<Body>>>::Future: Send + 'static,
	<S as Service<Request<Body>>>::Error: Debug,
{
	type Error = S::Error;
	type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;
	type Response = S::Response;

	fn call(&mut self, req: Request<Body>) -> Self::Future
	where
		<E::RequestBody as Preprocessable>::Processed: Send,
	{
		let (parts, body) = req.into_parts();

		if let Ok(_) = Cookie::parse_encoded(
			parts
				.headers
				.get(http::header::COOKIE)
				.and_then(|v| v.to_str().ok())
				.unwrap_or_default(),
		) {
			// TODO parse cookie and check if it's valid
			let req = Request::from_parts(parts, body);
			let future = self.inner.call(req);
			Box::pin(async move { future.await })
		} else {
			leptos_axum::redirect(&LoggedOutRoute::Login.to_string());

			Box::pin(async { Ok(Response::default()) })
		}
	}

	fn poll_ready(
		&mut self,
		cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Result<(), Self::Error>> {
		self.inner.poll_ready(cx)
	}
}
