use std::task::{Context, Poll};

use axum::extract::Request;
use tower::{Layer, Service};

use crate::prelude::*;

/// A [`tower::Layer`] that can be used to mount the workspace to the router
#[derive(Clone)]
pub struct WorkspaceLayer {
	state: AppState,
}

impl<S> Layer<S> for WorkspaceLayer {
	type Service = WorkspaceService<S>;

	fn layer(&self, inner: S) -> Self::Service {
		WorkspaceService {
			inner,
			state: self.state.clone(),
		}
	}
}

/// A [`tower::Service`] that can be check whether the workspace is valid.
#[derive(Clone)]
pub struct WorkspaceService<S> {
	inner: S,
	state: AppState,
}

impl<S, B> Service<Request<B>> for WorkspaceService<S>
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

		async move { inner.call(req).await }
	}
}
