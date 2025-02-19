use std::marker::PhantomData;

use axum_extra::routing::TypedPath;
use leptos_router::{
	components::ParentRoute,
	hooks::{use_params as use_router_params, use_query as use_router_query},
	params::Params,
};
use serde::{de::DeserializeOwned, Serialize};

use crate::prelude::*;

/// A trait for types that can be used as a route in the application.
/// It also provides the path as well as the query parameters for the route.
pub trait TypedRoute:
	TypedPath + Params + DeserializeOwned + Serialize + PartialEq + Clone + Send + Sync + 'static
{
	/// Whether the route requires the user to be logged in.
	const REQUIRES_LOGIN: bool;

	/// The query parameters for the route.
	type Query: Params
		+ DeserializeOwned
		+ Serialize
		+ PartialEq
		+ Clone
		+ Default
		+ Send
		+ Sync
		+ 'static;
}

#[component(transparent)]
pub fn AppRoute<R, F, V>(
	/// Phantom data for the route
	#[prop(optional)]
	_phantom: PhantomData<R>,
	/// The view for the route
	view: F,
	/// The Children of the route
	#[prop(optional, default = Box::new(|| ().into_any()))]
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

	view! {
		<ParentRoute view={move || view(query.clone(), params.clone())} path={<R as TypedPath>::PATH}>
			{children()}
		</ParentRoute>
	}
}
