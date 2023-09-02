use std::error::Error as StdError;

use axum::{
	body::HttpBody,
	routing::{MethodFilter, MethodRouter},
	Router,
};
use axum_extra::routing::TypedPath;
use models::{utils::AuthenticationType, ApiEndpoint};
use tower::ServiceBuilder;

use crate::{
	prelude::AppState,
	utils::layers::{
		AuthEndpointHandler,
		AuthEndpointLayer,
		AuthenticatedParserLayer,
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
		E: ApiEndpoint;

	/// Mount an API endpoint directly along with the required request parser,
	/// Rate limiter, Audit logger and Auth middlewares, using tower layers.
	#[track_caller]
	fn mount_auth_endpoint<E, H>(self, handler: H, state: AppState) -> Self
	where
		H: AuthEndpointHandler<E> + Clone + Send + 'static,
		E: ApiEndpoint;
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
		E: ApiEndpoint,
	{
		verify_endpoint_no_auth(E::AUTHENTICATION);
		use AuthenticationType as Auth;
		self.route(<<E as ApiEndpoint>::RequestPath as TypedPath>::PATH, {
			let router = MethodRouter::<S, B>::new().on(
				MethodFilter::try_from(<E as ApiEndpoint>::METHOD).unwrap(),
				|| async { unreachable!() },
			);
			match E::AUTHENTICATION {
				Auth::NoAuthentication => router.layer(
					ServiceBuilder::new()
						// .layer(todo!("Add rate limiter checker middleware here")),
						.layer(RequestParserLayer::with_state(state))
						// .layer(todo!("Add rate limiter value updater middleware here"))
						.layer(EndpointLayer::new(handler)),
				),
				Auth::PlainTokenAuthenticator |
				Auth::WorkspaceMembershipAuthenticator { .. } |
				Auth::ResourcePermissionAuthenticator { .. } => unreachable!(),
			}
		})
	}

	#[track_caller]
	fn mount_auth_endpoint<E, H>(self, handler: H, state: AppState) -> Self
	where
		H: AuthEndpointHandler<E> + Clone + Send + 'static,
		E: ApiEndpoint,
	{
		verify_endpoint_auth(E::AUTHENTICATION);
		use AuthenticationType as Auth;
		self.route(<<E as ApiEndpoint>::RequestPath as TypedPath>::PATH, {
			let router = MethodRouter::<S, B>::new().on(
				MethodFilter::try_from(<E as ApiEndpoint>::METHOD).unwrap(),
				|| async { unreachable!() },
			);
			match E::AUTHENTICATION {
				Auth::NoAuthentication => unreachable!(),
				Auth::PlainTokenAuthenticator => router.layer(
					ServiceBuilder::new()
						// .layer(todo!("Add rate limiter checker middleware here")),
						.layer(AuthenticatedParserLayer::with_state(state))
						// .layer(todo!("Add auth middleware here"))
						// .layer(todo!("Add rate limiter value updater middleware here"))
						// .layer(todo!("Add audit logger middleware here"))
						.layer(AuthEndpointLayer::new(handler)),
				),
				Auth::WorkspaceMembershipAuthenticator {
					extract_workspace_id,
				} => todo!(),
				Auth::ResourcePermissionAuthenticator {
					extract_resource_id,
				} => todo!(),
			}
		})
	}
}

const fn verify_endpoint_no_auth<E>(auth: AuthenticationType<E>)
where
	E: ApiEndpoint,
{
	use AuthenticationType as Auth;
	match auth {
		Auth::NoAuthentication => {}
		Auth::PlainTokenAuthenticator |
		Auth::ResourcePermissionAuthenticator { .. } |
		Auth::WorkspaceMembershipAuthenticator { .. } => panic!(concat!(
			"Endpoint with authentication cannot be mounted here. ",
			"Use mount_auth_endpoint instead"
		)),
	}
}

const fn verify_endpoint_auth<E>(auth: AuthenticationType<E>)
where
	E: ApiEndpoint,
{
	use AuthenticationType as Auth;
	match auth {
		Auth::NoAuthentication => panic!(concat!(
			"Endpoint without authentication cannot be mounted here. ",
			"Use mount_endpoint instead"
		)),
		Auth::PlainTokenAuthenticator |
		Auth::ResourcePermissionAuthenticator { .. } |
		Auth::WorkspaceMembershipAuthenticator { .. } => {}
	}
}
