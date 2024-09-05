use std::future::Future;

use models::ApiEndpoint;
use preprocess::Preprocessable;

use crate::prelude::*;

/// A trait that is implemented for functions and closures that take a specific
/// request and returns an async fn that returns a response. This can be used to
/// mount endpoint handlers that respond to specific API endpoints as per the
/// [`ApiEndpoint`] trait.
pub trait EndpointHandler<'req, E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// The future returned by the endpoint handler.
	type Future: Future<Output = Result<AppResponse<E>, ErrorType>> + Send;

	/// Call the endpoint handler with the given request.
	fn call(self, req: AppRequest<'req, E>) -> Self::Future;
}

impl<'req, F, Fut, E> EndpointHandler<'req, E> for F
where
	F: FnOnce(AppRequest<'req, E>) -> Fut,
	Fut: Future<Output = Result<AppResponse<E>, ErrorType>> + Send,
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	type Future = Fut;

	fn call(self, req: AppRequest<'req, E>) -> Self::Future {
		self(req)
	}
}
