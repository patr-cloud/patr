use std::{
	future::Future,
	marker::PhantomData,
	task::{Context, Poll},
};

use preprocess::Preprocessable;
use tower::{Layer, Service};

use crate::prelude::*;

/// The [`tower::Layer`] used to preprocess requests. This will parse the
/// use the [`preprocess`] crate to validate requests. All subsequent
/// underlying layers will recieve an [`AuthenticatedAppRequest`] with the
/// appropriate [`PreprocessApiRequest`].
pub struct LoginIdLayer<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// The endpoint type that this layer will handle.
	endpoint: PhantomData<E>,
}

impl<E> Default for LoginIdLayer<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	fn default() -> Self {
		Self::new()
	}
}

impl<E> LoginIdLayer<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// Helper function to initialize a new preprocess layer
	pub const fn new() -> Self {
		Self {
			endpoint: PhantomData,
		}
	}
}

impl<E, S> Layer<S> for LoginIdLayer<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
	for<'a> S: Service<AppRequest<'a, E>>,
{
	type Service = LoginIdService<E, S>;

	fn layer(&self, inner: S) -> Self::Service {
		LoginIdService {
			inner,
			endpoint: PhantomData,
		}
	}
}

impl<E> Clone for LoginIdLayer<E>
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

/// The underlying service that runs when the [`LoginIdLayer`] is used.
pub struct LoginIdService<E, S>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// The inner service that will be called after the request's login Id is
	/// handled
	inner: S,
	/// The endpoint type that this service will handle.
	endpoint: PhantomData<E>,
}

impl<'a, E, S> Service<AuthenticatedAppRequest<'a, E>> for LoginIdService<E, S>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
	for<'b> S: Service<AuthenticatedAppRequest<'b, E>, Response = AppResponse<E>, Error = ErrorType>
		+ Clone,
{
	type Error = ErrorType;
	type Response = AppResponse<E>;

	type Future = impl Future<Output = Result<Self::Response, Self::Error>>;

	fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		self.inner.poll_ready(cx)
	}

	#[instrument(skip(self, req), name = "LoginIdService")]
	fn call(&mut self, req: AuthenticatedAppRequest<'a, E>) -> Self::Future {
		let mut inner = self.inner.clone();
		async move {
			trace!("Preprocessing request");

			inner.call(req).await
		}
	}
}

impl<E, S> Clone for LoginIdService<E, S>
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
