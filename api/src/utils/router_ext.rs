use std::error::Error as StdError;

use axum::{
	body::HttpBody,
	routing::{MethodFilter, MethodRouter},
	Router,
};
use axum_extra::routing::TypedPath;
use models::{
	utils::{AppAuthentication, BearerToken, HasHeader, NoAuthentication},
	ApiEndpoint,
};
use tower::ServiceBuilder;

use super::layers::AuthenticationLayer;
use crate::{
	prelude::AppState,
	utils::layers::{
		AuthEndpointHandler,
		AuthEndpointLayer,
		EndpointHandler,
		EndpointLayer,
		RequestParserLayer,
	},
};

/// Extension trait for axum Router to mount an API endpoint directly along with
/// the required request parser, Rate limiter, Audit logger and Auth
/// middlewares, using tower layers.
pub trait RouterExt<S, B>
where
	B: HttpBody + Send + 'static,
	S: Clone + Send + Sync + 'static,
{
	/// Mount an API endpoint directly along with the required request parser,
	/// Rate limiter using tower layers.
	#[track_caller]
	fn mount_endpoint<E, H>(self, handler: H, state: AppState) -> Self
	where
		H: EndpointHandler<E> + Clone + Send + 'static,
		E: ApiEndpoint<Authenticator = NoAuthentication>;

	/// Mount an API endpoint directly along with the required request parser,
	/// Rate limiter, Audit logger and Auth middlewares, using tower layers.
	#[track_caller]
	fn mount_auth_endpoint<E, H>(self, handler: H, state: AppState) -> Self
	where
		H: AuthEndpointHandler<E> + Clone + Send + 'static,
		E: ApiEndpoint<Authenticator = AppAuthentication<E>>,
		E::RequestHeaders: HasHeader<BearerToken>;
}

impl<S, B> RouterExt<S, B> for Router<S, B>
where
	B: HttpBody + Send + 'static,
	<B as HttpBody>::Data: Send,
	<B as HttpBody>::Error: StdError + Send + Sync,
	S: Clone + Send + Sync + 'static,
{
	#[track_caller]
	fn mount_endpoint<E, H>(self, handler: H, state: AppState) -> Self
	where
		H: EndpointHandler<E> + Clone + Send + 'static,
		E: ApiEndpoint<Authenticator = NoAuthentication>,
	{
		self.route(
			<<E as ApiEndpoint>::RequestPath as TypedPath>::PATH,
			MethodRouter::<S, B>::new()
				.on(
					MethodFilter::try_from(<E as ApiEndpoint>::METHOD).unwrap(),
					|| async { unreachable!() },
				)
				.layer(
					ServiceBuilder::new()
						// .layer(todo!("Add rate limiter checker middleware here")),
						.layer(RequestParserLayer::with_state(state))
						// .layer(todo!("Add rate limiter value updater middleware here"))
						.layer(EndpointLayer::new(handler)),
				),
		)
	}

	#[track_caller]
	fn mount_auth_endpoint<E, H>(self, handler: H, state: AppState) -> Self
	where
		H: AuthEndpointHandler<E> + Clone + Send + 'static,
		E: ApiEndpoint<Authenticator = AppAuthentication<E>>,
		E::RequestHeaders: HasHeader<BearerToken>,
	{
		self.route(
			<<E as ApiEndpoint>::RequestPath as TypedPath>::PATH,
			MethodRouter::<S, B>::new()
				.on(
					MethodFilter::try_from(<E as ApiEndpoint>::METHOD).unwrap(),
					|| async { unreachable!() },
				)
				.layer(
					ServiceBuilder::new()
						// .layer(todo!("Add rate limiter checker middleware here")),
						.layer(RequestParserLayer::with_state(state))
						.layer(AuthenticationLayer::new())
						// .layer(todo!("Add rate limiter value updater middleware here"))
						// .layer(todo!("Add audit logger middleware here"))
						.layer(AuthEndpointLayer::new(handler)),
				),
		)
	}
}
