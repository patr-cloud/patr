use std::{
	convert::Infallible,
	task::{Context, Poll},
};

use axum::{RequestExt, body::Body, extract::Request, response::Response};
use tower::{Layer, Service};

use crate::{
	prelude::*,
	utils::{extractors::ClientIP, layers::RegistryRequest},
};

/// A [`tower::Layer`] that can be used to mount the workspace to the router
#[derive(Clone)]
pub struct RegistryDataStoreConnectionLayer {
	state: AppState,
}

impl RegistryDataStoreConnectionLayer {
	/// Create a new instance of the [`RegistryDataStoreConnectionLayer`] with
	/// the given state. This state will be used to parse the request, create a
	/// database transaction, and call the inner service. If the inner service
	/// fails, the database transaction will be automatically rolled back,
	/// otherwise it will be committed.
	pub fn with_state(state: AppState) -> Self {
		Self { state }
	}
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

impl<S> Service<Request<Body>> for RegistryDataStoreConnectionService<S>
where
	for<'a> S: Service<RegistryRequest<'a>, Response = Response, Error = Infallible> + Clone,
{
	type Error = Infallible;
	type Response = Response;

	type Future = impl Future<Output = Result<Self::Response, Self::Error>>;

	fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		self.inner
			.poll_ready(cx)
			.map_err(|_| unreachable!("Layers must always be ready"))
	}

	fn call(&mut self, mut req: Request<Body>) -> Self::Future {
		let mut inner = self.inner.clone();
		let state = self.state.clone();

		async move {
			let Ok(mut database) = state.database.begin().await else {
				debug!("Failed to begin database transaction");
				panic!();
			};

			let Ok(ClientIP(client_ip)) = req.extract_parts().await;

			let req = RegistryRequest {
				request: req,
				database: &mut database,
				client_ip,
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
