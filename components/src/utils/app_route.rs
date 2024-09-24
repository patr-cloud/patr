use std::marker::PhantomData;

use axum_extra::routing::TypedPath;
use leptos::*;
use leptos_router::{
	use_params as use_router_params,
	use_query as use_router_query,
	use_route_data,
	Params,
	Route,
};
use serde::{de::DeserializeOwned, Serialize};

/// A trait for types that can be used as a route in the application.
/// It also provides the path as well as the query parameters for the route.
pub trait TypedRoute:
	TypedPath + Params + DeserializeOwned + Serialize + PartialEq + Clone + 'static
{
	/// Whether the route requires the user to be logged in.
	const REQUIRES_LOGIN: bool;

	/// The query parameters for the route.
	type Query: Params + DeserializeOwned + Serialize + PartialEq + Clone + Default + 'static;
}

#[component(transparent)]
pub fn AppRoute<R, F, V>(
	/// Phantom data for the route
	#[prop(optional)]
	_phantom: PhantomData<R>,
	/// The view for the route
	view: F,
	/// The Children of the route
	#[prop(optional, default = Box::new(|| Fragment::new(vec![])))]
	children: Children,
) -> impl IntoView
where
	R: TypedRoute,
	F: Fn(R::Query, R) -> V + 'static,
	V: IntoView,
{
	let query: R::Query = use_router_query().get_untracked().unwrap_or_default();
	let params: R = use_router_params()
		.get_untracked()
		.expect("cannot parse params");
	let path = <R as TypedPath>::PATH.to_string();

	let current_path = use_route_data::<String>().unwrap_or_default();
	let router_path = path
		.trim_start_matches(&current_path)
		.trim_start_matches('/')
		.to_string();

	view! {
		<Route
			view={move || view(query.clone(), params.clone())}
			path={router_path}
			data={move || path.clone()}>
			{children()}
		</Route>
	}
}
