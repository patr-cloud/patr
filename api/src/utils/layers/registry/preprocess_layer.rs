use std::{
	future::Future,
	marker::PhantomData,
	task::{Context, Poll},
};

use preprocess::Preprocessable;
use tower::{Layer, Service};

use crate::routes::registry_patr_cloud::prelude::*;

/// The [`tower::Layer`] used to preprocess requests. This will parse the
/// use the [`preprocess`] crate to validate requests. All subsequent
/// underlying layers will recieve an [`AuthenticatedRegistryAppRequest`] with
/// the appropriate [`RegistryProcessedApiRequest`].
pub struct RegistryPreprocessLayer<E>
where
	E: RegistryEndpoint,
	<E::RequestPath as Preprocessable>::Processed: Send,
	<E::RequestQuery as Preprocessable>::Processed: Send,
{
	/// The endpoint type that this layer will handle.
	endpoint: PhantomData<E>,
}

impl<E> Default for RegistryPreprocessLayer<E>
where
	E: RegistryEndpoint,
	<E::RequestPath as Preprocessable>::Processed: Send,
	<E::RequestQuery as Preprocessable>::Processed: Send,
{
	fn default() -> Self {
		Self::new()
	}
}

impl<E> RegistryPreprocessLayer<E>
where
	E: RegistryEndpoint,
	<E::RequestPath as Preprocessable>::Processed: Send,
	<E::RequestQuery as Preprocessable>::Processed: Send,
{
	/// Helper function to initialize a new preprocess layer
	pub const fn new() -> Self {
		Self {
			endpoint: PhantomData,
		}
	}
}

impl<E, S> Layer<S> for RegistryPreprocessLayer<E>
where
	E: RegistryEndpoint,
	<E::RequestPath as Preprocessable>::Processed: Send,
	<E::RequestQuery as Preprocessable>::Processed: Send,
	for<'a> S: Service<RegistryAppRequest<'a, E>>,
{
	type Service = RegistryPreprocessService<E, S>;

	fn layer(&self, inner: S) -> Self::Service {
		RegistryPreprocessService {
			inner,
			endpoint: PhantomData,
		}
	}
}

impl<E> Clone for RegistryPreprocessLayer<E>
where
	E: RegistryEndpoint,
	<E::RequestPath as Preprocessable>::Processed: Send,
	<E::RequestQuery as Preprocessable>::Processed: Send,
{
	fn clone(&self) -> Self {
		Self {
			endpoint: PhantomData,
		}
	}
}

/// The underlying service that runs when the [`RegistryPreprocessLayer`] is
/// used.
pub struct RegistryPreprocessService<E, S>
where
	E: RegistryEndpoint,
	<E::RequestPath as Preprocessable>::Processed: Send,
	<E::RequestQuery as Preprocessable>::Processed: Send,
{
	/// The inner service that will be called after the request is processed.
	inner: S,
	/// The endpoint type that this service will handle.
	endpoint: PhantomData<E>,
}

impl<'a, E, S> Service<RegistryUnprocessedAppRequest<'a, E>> for RegistryPreprocessService<E, S>
where
	E: RegistryEndpoint,
	<E::RequestPath as Preprocessable>::Processed: Send,
	<E::RequestQuery as Preprocessable>::Processed: Send,
	for<'b> S: Service<RegistryAppRequest<'b, E>, Response = RegistryResponse<E>, Error = RegistryError>
		+ Clone,
{
	type Error = RegistryError;
	type Response = RegistryResponse<E>;

	type Future = impl Future<Output = Result<Self::Response, Self::Error>>;

	fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		self.inner.poll_ready(cx)
	}

	#[instrument(skip(self, req), name = "PreprocessService")]
	fn call(&mut self, req: RegistryUnprocessedAppRequest<'a, E>) -> Self::Future {
		let mut inner = self.inner.clone();
		async move {
			trace!("Preprocessing request");

			let RegistryUnprocessedAppRequest {
				request,
				database,
				redis,
				client_ip,
				s3,
				config,
			} = req;
			let req = RegistryAppRequest {
				request: RegistryProcessedApiRequest::try_from(request).map_err(
					|err: preprocess::Error| {
						info!(
							"Error processing request: field `{}` is invalid: {}",
							err.field, err.message
						);
						RegistryError::builder()
							.code(ErrorCode::Unsupported)
							.message(format!(
								"Invalid request: field `{}` is invalid: {}",
								err.field, err.message
							))
							.status(StatusCode::BAD_REQUEST)
							.build()
					},
				)?,
				database,
				redis,
				client_ip,
				s3,
				config,
			};
			inner.call(req).await
		}
	}
}

impl<E, S> Clone for RegistryPreprocessService<E, S>
where
	E: RegistryEndpoint,
	<E::RequestPath as Preprocessable>::Processed: Send,
	<E::RequestQuery as Preprocessable>::Processed: Send,
	for<'b> S: Service<RegistryAppRequest<'b, E>, Response = RegistryResponse<E>, Error = RegistryError>
		+ Clone,
{
	fn clone(&self) -> Self {
		Self {
			inner: self.inner.clone(),
			endpoint: PhantomData,
		}
	}
}
