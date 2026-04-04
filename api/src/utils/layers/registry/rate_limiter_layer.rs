use std::{
	future::Future,
	marker::PhantomData,
	task::{Context, Poll},
	time::Duration,
};

use preprocess::Preprocessable;
use tower::{Layer, Service};

use crate::{models::rate_limiter::check_rate_limit, routes::registry_patr_cloud::prelude::*};

/// The global rate limit windows applied to all endpoints.
/// Each tuple is (max_requests, window_duration).
/// Registry limits are more generous than API limits because a single
/// `docker pull` or `docker push` generates many parallel HTTP requests
/// (manifest + config + layer blobs). These limits are roughly 2x Docker Hub's
/// effective HTTP request rate for authenticated free-tier users.
const RATE_LIMITS: [(u32, Duration); 3] = [
	(30, Duration::from_secs(1)),
	(300, Duration::from_secs(60)),
	(1000, Duration::from_secs(3600)),
];

/// Tower layer that applies both per-IP and per-login rate limiting to
/// registry endpoints. Operates on [`AuthenticatedRegistryAppRequest`] after
/// authentication has been verified.
pub struct RegistryRateLimiterLayer<E>
where
	E: RegistryEndpoint,
{
	endpoint: PhantomData<E>,
}

impl<E> Default for RegistryRateLimiterLayer<E>
where
	E: RegistryEndpoint,
{
	fn default() -> Self {
		Self::new()
	}
}

impl<E> RegistryRateLimiterLayer<E>
where
	E: RegistryEndpoint,
{
	/// Helper function to initialize a registry rate limiter layer
	pub const fn new() -> Self {
		Self {
			endpoint: PhantomData,
		}
	}
}

impl<S, E> Layer<S> for RegistryRateLimiterLayer<E>
where
	E: RegistryEndpoint,
{
	type Service = RegistryRateLimiterService<S, E>;

	fn layer(&self, inner: S) -> Self::Service {
		RegistryRateLimiterService {
			inner,
			endpoint: PhantomData,
		}
	}
}

impl<E> Clone for RegistryRateLimiterLayer<E>
where
	E: RegistryEndpoint,
{
	fn clone(&self) -> Self {
		Self {
			endpoint: PhantomData,
		}
	}
}

/// The underlying service for [`RegistryRateLimiterLayer`].
pub struct RegistryRateLimiterService<S, E>
where
	E: RegistryEndpoint,
{
	inner: S,
	endpoint: PhantomData<E>,
}

impl<'a, S, E> Service<AuthenticatedRegistryAppRequest<'a, E>> for RegistryRateLimiterService<S, E>
where
	for<'b> S: Service<
			AuthenticatedRegistryAppRequest<'b, E>,
			Response = RegistryResponse<E>,
			Error = RegistryError,
		> + Clone
		+ 'a,
	E: RegistryEndpoint,
	<E::RequestPath as Preprocessable>::Processed: Send,
	<E::RequestQuery as Preprocessable>::Processed: Send,
{
	type Error = RegistryError;
	type Response = RegistryResponse<E>;

	type Future = impl Future<Output = Result<Self::Response, Self::Error>> + 'a;

	fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		self.inner
			.poll_ready(cx)
			.map_err(|_| unreachable!("Layers must always be ready"))
	}

	#[instrument(skip(self, req), name = "RegistryRateLimiterService")]
	fn call(&mut self, req: AuthenticatedRegistryAppRequest<'a, E>) -> Self::Future {
		let mut inner = self.inner.clone();
		Box::pin(async move {
			check_rate_limit(
				req.redis,
				req.client_ip,
				Some(&req.user_data.login_id),
				&RATE_LIMITS,
			)
			.await
			.map_err(|_| {
				RegistryError::builder()
					.code(ErrorCode::TooManyRequests)
					.message("Too many requests. Please try again later.")
					.status(StatusCode::TOO_MANY_REQUESTS)
					.build()
			})?;

			inner.call(req).await
		})
	}
}

impl<S, E> Clone for RegistryRateLimiterService<S, E>
where
	for<'b> S: Service<
			AuthenticatedRegistryAppRequest<'b, E>,
			Response = RegistryResponse<E>,
			Error = RegistryError,
		> + Clone,
	E: RegistryEndpoint,
{
	fn clone(&self) -> Self {
		Self {
			inner: self.inner.clone(),
			endpoint: PhantomData,
		}
	}
}
