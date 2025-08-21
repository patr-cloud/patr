use std::{
	future::Future,
	marker::PhantomData,
	net::IpAddr,
	task::{Context, Poll},
};

use models::prelude::*;
use preprocess::Preprocessable;
use tower::{Layer, Service};

use crate::prelude::*;

/// A [`tower::Layer`] that can be used to parse the request and call the inner
/// service with the parsed request. Ideally, this will automatically be done by
/// [`RouterExt::mount_endpoint`], and you should not need to use this directly.
#[derive(Clone, Debug)]
pub struct DataStoreConnectionLayer<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// The state that will be used to parse the request, create a database
	/// transaction and a redis connection, and call the inner service. If the
	/// inner service fails, the database transaction will be automatically
	/// rolled back, otherwise it will be committed.
	state: AppState,
	/// The endpoint type that this layer will handle.
	phantom: PhantomData<E>,
}

impl<E> DataStoreConnectionLayer<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// Create a new instance of the [`RequestParserLayer`] with the given
	/// state. This state will be used to parse the request, create a database
	/// transaction, and call the inner service. If the inner service fails, the
	/// database transaction will be automatically rolled back, otherwise it
	/// will be committed.
	pub fn with_state(state: AppState) -> Self {
		Self {
			phantom: PhantomData,
			state,
		}
	}
}

impl<S, E> Layer<S> for DataStoreConnectionLayer<E>
where
	for<'a> S: Service<UnprocessedAppRequest<'a, E>>,
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	type Service = DataStoreConnectionService<S, E>;

	fn layer(&self, inner: S) -> Self::Service {
		DataStoreConnectionService {
			inner,
			state: self.state.clone(),
			phantom: PhantomData,
		}
	}
}

/// A [`tower::Service`] that can be used to parse the request and call the
/// inner service with the parsed request. Ideally, this will automatically be
/// done by [`RouterExt::mount_endpoint`], and you should not need to use this
/// directly.
#[derive(Clone, Debug)]
pub struct DataStoreConnectionService<S, E>
where
	for<'a> S: Service<UnprocessedAppRequest<'a, E>>,
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// The inner service that will be called with the parsed request.
	inner: S,
	/// The state that will be used to parse the request, create a database
	/// transaction and a redis connection, and call the inner service. If the
	/// inner service fails, the database transaction will be automatically
	/// rolled back, otherwise it will be committed.
	state: AppState,
	/// The endpoint type that this service will handle.
	phantom: PhantomData<E>,
}

impl<S, E> Service<(ApiRequest<E>, IpAddr)> for DataStoreConnectionService<S, E>
where
	for<'a> S:
		Service<UnprocessedAppRequest<'a, E>, Response = AppResponse<E>, Error = ErrorType> + Clone,
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	type Error = ErrorType;
	type Response = AppResponse<E>;

	type Future = impl Future<Output = Result<Self::Response, Self::Error>>;

	fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		self.inner
			.poll_ready(cx)
			.map_err(|_| unreachable!("Layers must always be ready"))
	}

	#[instrument(skip(self, request), name = "DataStoreConnectionService")]
	fn call(&mut self, (request, client_ip): (ApiRequest<E>, IpAddr)) -> Self::Future {
		let mut state = self.state.clone();
		let mut inner = self.inner.clone();
		async {
			let redis = &mut state.redis;

			let Ok(mut database) = state.database.begin().await else {
				debug!("Failed to begin database transaction");
				return Err(ErrorType::server_error(
					"unable to begin database transaction",
				));
			};

			let req = UnprocessedAppRequest {
				request,
				database: &mut database,
				redis,
				client_ip,
				config: state.config.clone(),
			};

			info!("Calling inner service");

			match inner.call(req).await {
				Ok(response) => {
					info!("Inner service called successfully");
					let Ok(()) = database.commit().await else {
						debug!("Failed to commit database transaction");
						return Err(ErrorType::server_error(
							"unable to commit database transaction",
						));
					};
					Ok(response)
				}
				Err(error) => {
					warn!("Inner service failed: {:?}", error);
					let Ok(()) = database.rollback().await else {
						debug!("Failed to rollback database transaction");
						return Err(ErrorType::server_error(
							"unable to rollback database transaction",
						));
					};
					Err(error)
				}
			}
		}
	}
}
