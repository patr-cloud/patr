use std::{
	future::Future,
	marker::PhantomData,
	task::{Context, Poll},
};

use preprocess::Preprocessable;
use tower::{Layer, Service};

use crate::prelude::*;

/// The [`tower::Layer`] used to validate that the user agent is from a browser.
/// This will parse the [`UserAgent`][headers::UserAgent] header and verify it
/// is that of a browser. If it is not that of a browser, the request will be
/// rejected
pub struct UserAgentValidationLayer<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// The endpoint type that this layer will handle.
	endpoint: PhantomData<E>,
}

impl<E> Default for UserAgentValidationLayer<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	fn default() -> Self {
		Self::new()
	}
}

impl<E> UserAgentValidationLayer<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// Helper function to initialize a user agent validation layer
	pub const fn new() -> Self {
		Self {
			endpoint: PhantomData,
		}
	}
}

impl<E, S> Layer<S> for UserAgentValidationLayer<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
	for<'a> S: Service<AppRequest<'a, E>>,
{
	type Service = UserAgentValidationService<E, S>;

	fn layer(&self, inner: S) -> Self::Service {
		UserAgentValidationService {
			inner,
			endpoint: PhantomData,
		}
	}
}

impl<E> Clone for UserAgentValidationLayer<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	fn clone(&self) -> Self {
		Self {
			endpoint: PhantomData,
		}
	}
}

/// The underlying service that runs when the [`UserAgentValidationLayer`] is
/// used.
pub struct UserAgentValidationService<E, S>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// The inner service that will be called with the validated user agent.
	inner: S,
	/// The endpoint type that this service will handle.
	endpoint: PhantomData<E>,
}

impl<'a, E, S> Service<AppRequest<'a, E>> for UserAgentValidationService<E, S>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
	for<'b> S: Service<AppRequest<'b, E>, Response = AppResponse<E>, Error = ErrorType> + Clone,
{
	type Error = ErrorType;
	type Response = AppResponse<E>;

	type Future = impl Future<Output = Result<Self::Response, Self::Error>>;

	fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		self.inner.poll_ready(cx)
	}

	#[instrument(skip(self, req), name = "UserAgentValidationService")]
	fn call(&mut self, req: AppRequest<'a, E>) -> Self::Future {
		let mut inner = self.inner.clone();
		async move {
			trace!("Validating user agent");
			inner.call(req).await
		}
	}
}

impl<E, S> Clone for UserAgentValidationService<E, S>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
	for<'b> S: Service<AppRequest<'b, E>, Response = AppResponse<E>, Error = ErrorType> + Clone,
{
	fn clone(&self) -> Self {
		Self {
			inner: self.inner.clone(),
			endpoint: PhantomData,
		}
	}
}
