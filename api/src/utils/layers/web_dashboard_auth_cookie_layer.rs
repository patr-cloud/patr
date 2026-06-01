use std::{
	convert::Infallible,
	future::Future,
	task::{Context, Poll},
};

use axum::{body::Body, http::Request, response::Response};
use headers::{Authorization, Cookie, HeaderMapExt, authorization::Credentials};
use http::header;
use models::prelude::*;
use serde::{Deserialize, Serialize};
use tower::{Layer, Service};
use tracing::Span;

/// The authentication state extracted from the auth state cookie.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AuthState {
	/// The user is not logged in.
	LoggedOut,
	/// The user is logged in with the given access and refresh tokens.
	#[serde(rename_all = "camelCase")]
	LoggedIn {
		/// The access token for the session.
		access_token: String,
		/// The refresh token for the session.
		refresh_token: String,
	},
}

/// A [`tower::Layer`] that converts authentication cookies into bearer tokens
/// for the web dashboard. This layer extracts the access token from the auth
/// state cookie (used by `app.patr.cloud`) and injects it as a bearer token
/// header, allowing downstream authentication layers to process it uniformly.
///
/// This enables the web dashboard to use cookie-based authentication while
/// reusing the same bearer token authentication logic as the API.
#[derive(Clone, Debug, Default)]
pub struct WebDashboardAuthCookieLayer;

impl WebDashboardAuthCookieLayer {
	/// Create a new instance of the [`WebDashboardAuthCookieLayer`].
	pub const fn new() -> Self {
		Self
	}
}

impl<S> Layer<S> for WebDashboardAuthCookieLayer
where
	for<'a> S: Service<Request<Body>>,
{
	type Service = WebDashboardAuthCookieService<S>;

	fn layer(&self, inner: S) -> Self::Service {
		WebDashboardAuthCookieService { inner }
	}
}

/// A [`tower::Service`] that extracts access tokens from auth state cookies
/// and converts them to bearer token headers for downstream processing.
///
/// This service performs the following steps:
/// 1. Extracts the auth state cookie from the incoming request
/// 2. Parses the access token from the cookie value
/// 3. Injects the access token as a bearer token authorization header
/// 4. Calls the inner service with the parsed request
/// 5. Returns the response back to the client
#[derive(Clone, Debug)]
pub struct WebDashboardAuthCookieService<S>
where
	S: Service<Request<Body>>,
{
	/// The inner service that will be called with the parsed request.
	inner: S,
}

impl<S> Service<Request<Body>> for WebDashboardAuthCookieService<S>
where
	S: Service<Request<Body>, Response = Response, Error = Infallible> + Clone,
{
	type Error = Infallible;
	type Response = Response;

	type Future = impl Future<Output = Result<Self::Response, Self::Error>>;

	fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		self.inner
			.poll_ready(cx)
			.map_err(|_| unreachable!("Layers must always be ready"))
	}

	#[instrument(name = "RequestParserService", skip(self, req), fields(
		http.request.auth_header_injected = %false
	))]
	fn call(&mut self, mut req: Request<Body>) -> Self::Future {
		let mut inner = self.inner.clone();
		async {
			// If the request already carries an Authorization header (e.g.
			// /auth/access-token sends `Bearer {refresh_token}`), don't
			// overwrite it — the caller intentionally chose a different
			// credential than the access token in the authState cookie.
			if req.headers().contains_key(header::AUTHORIZATION) {
				trace!("Authorization header present; not injecting from cookie");
				return inner.call(req).await;
			}

			let cookie = req
				.headers()
				.typed_get::<Cookie>()
				.as_ref()
				.and_then(|cookie| {
					cookie.get("authState") // This is the name of the cookie used by the frontend
				})
				.and_then(|cookie| serde_json::from_str::<AuthState>(cookie).ok());

			if let Some(AuthState::LoggedIn {
				access_token,
				refresh_token: _,
			}) = cookie
			{
				debug!("Injecting Authorization header with access token from authState cookie");

				if let Ok(access_token) = Authorization::bearer(&access_token).inspect_err(|err| {
					warn!("Failed to create Authorization header from access token: {err}");
				}) {
					req.headers_mut()
						.insert(header::AUTHORIZATION, access_token.0.encode());
					Span::current().record("http.request.auth_header_injected", true);
				}
			} else {
				warn!("No valid authState cookie found; proceeding without Authorization header");
			}

			info!("Calling inner service");

			inner.call(req).await
		}
	}
}
