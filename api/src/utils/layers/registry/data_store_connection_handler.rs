use std::task::{Context, Poll};

use axum::extract::Request;
use tower::{Layer, Service};

use crate::prelude::*;

/// A [`tower::Layer`] that can be used to mount the workspace to the router
#[derive(Clone)]
pub struct RegistryDataStoreConnectionLayer {
	state: AppState,
}

impl<S> Layer<S> for RegistryDataStoreConnectionLayer {
	type Service = RegistryDataStoreConnectionService<S>;

	fn layer(&self, inner: S) -> Self::Service {
		RegistryDataStoreConnectionService {
			inner,
			state: self.state.clone(),
		}
	}
}

/// A [`tower::Service`] that can be check whether the workspace is valid.
#[derive(Clone)]
pub struct RegistryDataStoreConnectionService<S> {
	inner: S,
	state: AppState,
}

impl<S, B> Service<Request<B>> for RegistryDataStoreConnectionService<S>
where
	S: Service<Request<B>> + Clone,
{
	type Error = S::Error;
	type Response = S::Response;

	type Future = impl Future<Output = Result<Self::Response, Self::Error>>;

	fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		self.inner
			.poll_ready(cx)
			.map_err(|_| unreachable!("Layers must always be ready"))
	}

	fn call(&mut self, req: Request<B>) -> Self::Future {
		let mut inner = self.inner.clone();
		let state = self.state.clone();

		async move {
			let Ok(database) = state.database.begin().await else {
				debug!("Failed to begin database transaction");
				panic!();
			};

			info!("Calling inner Service");
			match inner.call(req).await {
				Ok(response) => {
					info!("Inner service called successfully");
					let Ok(()) = database.commit().await else {
						debug!("Failed to commit database transaction");
						panic!();
					};
					Ok(response)
				}
				Err(error) => {
					warn!("Inner service failed");
					let Ok(()) = database.rollback().await else {
						debug!("Failed to rollback database transaction");
						panic!();
					};
					Err(error)
				}
			}
		}
	}
}
