use std::{
	future::Future,
	marker::PhantomData,
	task::{Context, Poll},
};

use models::prelude::*;
use preprocess::Preprocessable;
use tower::{Layer, Service};

use crate::{app::UnprocessedAppRequest, prelude::*};

/// A [`tower::Layer`] that can be used to parse the request and call the inner
/// service with the parsed request. Ideally, this will automatically be done by
/// [`RouterExt::mount_endpoint`], and you should not need to use this directly.
#[derive(Clone)]
pub struct DataStoreConnectionLayer<E, R>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
	R: RunnerExecutor,
{
	/// The state that will be used to parse the request, create a database
	/// transaction, and call the inner service. If the
	/// inner service fails, the database transaction will be automatically
	/// rolled back, otherwise it will be committed.
	state: AppState<R>,
	/// The endpoint type that this layer will handle.
	phantom: PhantomData<E>,
}

impl<E, R> DataStoreConnectionLayer<E, R>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
	R: RunnerExecutor,
{
	/// Create a new instance of the [`RequestParserLayer`] with the given
	/// state. This state will be used to parse the request, create a database
	/// transaction, and call the inner service. If the inner service fails, the
	/// database transaction will be automatically rolled back, otherwise it
	/// will be committed.
	pub fn with_state(state: AppState<R>) -> Self {
		Self {
			phantom: PhantomData,
			state,
		}
	}
}

impl<S, E, R> Layer<S> for DataStoreConnectionLayer<E, R>
where
	for<'a> S: Service<UnprocessedAppRequest<'a, E>>,
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
	R: RunnerExecutor,
{
	type Service = DataStoreConnectionService<S, E, R>;

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
#[derive(Clone)]
pub struct DataStoreConnectionService<S, E, R>
where
	for<'a> S: Service<UnprocessedAppRequest<'a, E>>,
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
	R: RunnerExecutor,
{
	/// The inner service that will be called with the parsed request.
	inner: S,
	/// The state that will be used to parse the request, create a database
	/// transaction, and call the inner service. If the
	/// inner service fails, the database transaction will be automatically
	/// rolled back, otherwise it will be committed.
	state: AppState<R>,
	/// The endpoint type that this service will handle.
	phantom: PhantomData<E>,
}

impl<S, E, R> Service<ApiRequest<E>> for DataStoreConnectionService<S, E, R>
where
	for<'a> S:
		Service<UnprocessedAppRequest<'a, E>, Response = AppResponse<E>, Error = ErrorType> + Clone,
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
	R: RunnerExecutor,
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
	fn call(&mut self, request: ApiRequest<E>) -> Self::Future {
		let state = self.state.clone();
		let mut inner = self.inner.clone();
		async {
			let Ok(mut database) = state.database.begin().await else {
				debug!("Failed to begin database transaction");
				return Err(ErrorType::server_error(
					"unable to begin database transaction",
				));
			};

			let req = UnprocessedAppRequest {
				request,
				database: &mut database,
				runner_changes_sender: state.runner_changes_sender.clone(),
				config: state.config.clone().into_base(),
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
