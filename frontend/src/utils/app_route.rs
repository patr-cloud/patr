use std::marker::PhantomData;

use axum_extra::routing::TypedPath;
use leptos::*;
use leptos_router::{
	use_params as use_router_params,
	use_query as use_router_query,
	Params,
	ProtectedRoute,
};
use serde::{de::DeserializeOwned, Serialize};

use crate::{pages::WorkspaceSidebarComponent, prelude::*};

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
	F: Fn(R, R::Query) -> V + Clone + 'static,
	V: IntoView,
{
	let query: R::Query = use_router_query().get_untracked().unwrap_or_default();
	let params: R = use_router_params()
		.get_untracked()
		.expect("cannot parse params");

	let redirect_path = if R::REQUIRES_LOGIN {
		format!("{}", LoginRoute {})
	} else {
		DeploymentsDashboardRoute {}.to_string()
	};

	let (state, _) = AuthState::load();
	let app_type = expect_context::<AppType>();

	view! {
		<ProtectedRoute
			view={move || {
				let view = view.clone();
				let query = query.clone();
				let params = params.clone();

				if state.get().is_logged_in() {
					view! {
						<div class="fr-fs-fs full-width full-height bg-secondary">
							<Sidebar sidebar_items={get_sidebar_items(
								app_type,
							)}>
								{app_type
									.is_managed()
									.then(|| {
										view! {
											<Transition>
												<WorkspaceSidebarComponent />
											</Transition>
										}
									})}
							</Sidebar>

							<main class="fc-fs-ct full-width px-lg">
								{view(params, query)}
							</main>
						</div>
					}
					.into_view()
				} else {
					view! {
						<PageContainer class="bg-image">
							{view(params, query)}
						</PageContainer>
					}
					.into_view()
				}
			}}
			path={<R as TypedPath>::PATH.to_string()}
			redirect_path={redirect_path}
			condition={|| !R::REQUIRES_LOGIN} >
			{children()}
		</ProtectedRoute>
	}
}
