use axum_extra::routing::TypedPath;
use leptos_router::{PossibleRouteMatch, params::Params};
use serde::{Serialize, de::DeserializeOwned};

/// A trait for types that can be used as a route in the application.
/// It also provides the path as well as the query parameters for the route.
pub trait TypedRoute {
	/// Whether the route requires the user to be logged in.
	const REQUIRES_LOGIN: bool;

	/// The URL path that is accepted by the route.
	type Path: TypedPath
		+ Params
		+ Serialize
		+ DeserializeOwned
		+ PartialEq
		+ Clone
		+ Send
		+ Sync
		+ 'static;

	/// The URL path that leptos accept (like with the [`path!`][1] macro). This
	/// is automatically generated from the URL that is passed to the
	/// [`TypedPath`][2] derive macro.
	///
	/// [1]: leptos_router::path
	/// [2]: axum_extra::routing::TypedPath
	#[doc(hidden)]
	fn leptos_path() -> impl PossibleRouteMatch + Clone + Send + Sync + 'static;

	/// The query parameters for the route.
	type Query: Params
		+ Serialize
		+ DeserializeOwned
		+ PartialEq
		+ Clone
		+ Default
		+ Send
		+ Sync
		+ 'static;
}
