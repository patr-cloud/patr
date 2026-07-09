use std::{
	future::Future,
	marker::PhantomData,
	task::{Context, Poll},
	time::Duration,
};

use preprocess::Preprocessable;
use tower::{Layer, Service};

use crate::{models::rate_limiter::check_rate_limit, prelude::*};

/// The global rate limit windows applied to all endpoints.
/// Each tuple is (max_requests, window_duration).
const RATE_LIMITS: [(u32, Duration); 3] = if cfg!(debug_assertions) {
	// Loose enough that SPA page-load bursts in the e2e suite (~8 parallel
	// queries per route mount, plus retries) never trip it; tight enough that
	// the rate-limit tests can exhaust the window within one second on a slow
	// CI runner. Tests pin these values — update them together.
	[
		(50, Duration::from_secs(1)),
		(2500, Duration::from_secs(60)),
		(25000, Duration::from_secs(3600)),
	]
} else {
	[
		(20, Duration::from_secs(1)),
		(500, Duration::from_secs(60)),
		(5000, Duration::from_secs(3600)),
	]
};

/// Tower layer that applies per-IP rate limiting to unauthenticated API
/// endpoints. Operates on [`UnprocessedAppRequest`] after the database
/// connection has been established.
pub struct RateLimiterLayer<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	endpoint: PhantomData<E>,
}

impl<E> Default for RateLimiterLayer<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	fn default() -> Self {
		Self::new()
	}
}

impl<E> RateLimiterLayer<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// Helper function to initialize a rate limiter layer
	pub const fn new() -> Self {
		Self {
			endpoint: PhantomData,
		}
	}
}

impl<E, S> Layer<S> for RateLimiterLayer<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
	for<'a> S: Service<UnprocessedAppRequest<'a, E>>,
{
	type Service = RateLimiterService<E, S>;

	fn layer(&self, inner: S) -> Self::Service {
		RateLimiterService {
			inner,
			endpoint: PhantomData,
		}
	}
}

impl<E> Clone for RateLimiterLayer<E>
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

/// The underlying service for [`RateLimiterLayer`].
pub struct RateLimiterService<E, S>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	inner: S,
	endpoint: PhantomData<E>,
}

impl<'a, E, S> Service<UnprocessedAppRequest<'a, E>> for RateLimiterService<E, S>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
	for<'b> S:
		Service<UnprocessedAppRequest<'b, E>, Response = AppResponse<E>, Error = ErrorType> + Clone,
{
	type Error = ErrorType;
	type Response = AppResponse<E>;

	type Future = impl Future<Output = Result<Self::Response, Self::Error>>;

	fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		self.inner
			.poll_ready(cx)
			.map_err(|_| unreachable!("Layers must always be ready"))
	}

	#[instrument(skip(self, req), name = "RateLimiterService")]
	fn call(&mut self, req: UnprocessedAppRequest<'a, E>) -> Self::Future {
		let mut inner = self.inner.clone();
		Box::pin(async move {
			check_rate_limit(req.redis, req.client_ip, None, &RATE_LIMITS).await?;

			inner.call(req).await
		})
	}
}

impl<E, S> Clone for RateLimiterService<E, S>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
	for<'b> S:
		Service<UnprocessedAppRequest<'b, E>, Response = AppResponse<E>, Error = ErrorType> + Clone,
{
	fn clone(&self) -> Self {
		Self {
			inner: self.inner.clone(),
			endpoint: PhantomData,
		}
	}
}

/// Tower layer that applies both per-IP and per-login rate limiting to
/// authenticated API endpoints. Operates on [`AuthenticatedAppRequest`] after
/// authentication and authorization have been verified.
pub struct AuthRateLimiterLayer<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	endpoint: PhantomData<E>,
}

impl<E> Default for AuthRateLimiterLayer<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	fn default() -> Self {
		Self::new()
	}
}

impl<E> AuthRateLimiterLayer<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// Helper function to initialize an auth rate limiter layer
	pub const fn new() -> Self {
		Self {
			endpoint: PhantomData,
		}
	}
}

impl<E, S> Layer<S> for AuthRateLimiterLayer<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
	for<'a> S: Service<AuthenticatedAppRequest<'a, E>>,
{
	type Service = AuthRateLimiterService<E, S>;

	fn layer(&self, inner: S) -> Self::Service {
		AuthRateLimiterService {
			inner,
			endpoint: PhantomData,
		}
	}
}

impl<E> Clone for AuthRateLimiterLayer<E>
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

/// The underlying service for [`AuthRateLimiterLayer`].
pub struct AuthRateLimiterService<E, S>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	inner: S,
	endpoint: PhantomData<E>,
}

impl<'a, E, S> Service<AuthenticatedAppRequest<'a, E>> for AuthRateLimiterService<E, S>
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
		self.inner
			.poll_ready(cx)
			.map_err(|_| unreachable!("Layers must always be ready"))
	}

	#[instrument(skip(self, req), name = "AuthRateLimiterService")]
	fn call(&mut self, req: AuthenticatedAppRequest<'a, E>) -> Self::Future {
		let mut inner = self.inner.clone();
		Box::pin(async move {
			check_rate_limit(
				req.redis,
				req.client_ip,
				Some(&req.user_data.login_id),
				&RATE_LIMITS,
			)
			.await?;

			inner.call(req).await
		})
	}
}

impl<E, S> Clone for AuthRateLimiterService<E, S>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
	for<'b> S: Service<AuthenticatedAppRequest<'b, E>, Response = AppResponse<E>, Error = ErrorType>
		+ Clone,
{
	fn clone(&self) -> Self {
		Self {
			inner: self.inner.clone(),
			endpoint: PhantomData,
		}
	}
}
