use std::{
	future::Future,
	marker::PhantomData,
	net::IpAddr,
	task::{Context, Poll},
};

use models::prelude::*;
use preprocess::Preprocessable;
use tower::{Layer, Service};

/// A [`tower::Layer`] that can be used to parse the request and call the inner
/// service with the parsed request. Ideally, this will automatically be done by
/// [`RouterExt::mount_endpoint`], and you should not need to use this directly.
pub struct RemoveIpAddrLayer<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// The endpoint type that this layer will handle.
	phantom: PhantomData<E>,
}

impl<E> RemoveIpAddrLayer<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// Create a new instance of the [`RequestParserLayer`] with the given
	/// state. This state will be used to parse the request, create a database
	/// transaction, and call the inner service. If the inner service fails, the
	/// database transaction will be automatically rolled back, otherwise it
	/// will be committed.
	pub const fn new() -> Self {
		Self {
			phantom: PhantomData,
		}
	}
}

impl<S, E> Layer<S> for RemoveIpAddrLayer<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
	for<'a> S: Service<ApiRequest<E>, Response = AppResponse<E>, Error = ErrorType> + Clone,
{
	type Service = RemoveIpAddrService<S, E>;

	fn layer(&self, inner: S) -> Self::Service {
		RemoveIpAddrService {
			inner,
			phantom: PhantomData,
		}
	}
}

impl<E> Clone for RemoveIpAddrLayer<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	fn clone(&self) -> Self {
		Self {
			phantom: PhantomData,
		}
	}
}

/// A [`tower::Service`] that can be used to parse the request and call the
/// inner service with the parsed request. Ideally, this will automatically be
/// done by [`RouterExt::mount_endpoint`], and you should not need to use this
/// directly.
pub struct RemoveIpAddrService<S, E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
	for<'a> S: Service<ApiRequest<E>, Response = AppResponse<E>, Error = ErrorType> + Clone,
{
	/// The inner service that will be called with the parsed request.
	inner: S,
	/// The endpoint type that this service will handle.
	phantom: PhantomData<E>,
}

impl<S, E> Service<(ApiRequest<E>, IpAddr)> for RemoveIpAddrService<S, E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
	for<'a> S: Service<ApiRequest<E>, Response = AppResponse<E>, Error = ErrorType> + Clone,
{
	type Error = ErrorType;
	type Response = AppResponse<E>;

	type Future = impl Future<Output = Result<Self::Response, Self::Error>>;

	fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		self.inner
			.poll_ready(cx)
			.map_err(|_| unreachable!("Layers must always be ready"))
	}

	#[instrument(skip(self, request), name = "RemoveIpAddrService")]
	fn call(&mut self, (request, _): (ApiRequest<E>, IpAddr)) -> Self::Future {
		let mut inner = self.inner.clone();
		async move { inner.call(request).await }
	}
}

impl<S, E> Clone for RemoveIpAddrService<S, E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
	for<'a> S: Service<ApiRequest<E>, Response = AppResponse<E>, Error = ErrorType> + Clone,
{
	fn clone(&self) -> Self {
		Self {
			inner: self.inner.clone(),
			phantom: PhantomData,
		}
	}
}
