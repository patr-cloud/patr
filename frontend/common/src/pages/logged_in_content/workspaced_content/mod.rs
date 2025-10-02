/// The Deployments set of page, contains, create, list, and update deployments
/// pages
mod deployment;
/// The Home page
mod home;

use leptos_router::{
	MatchNestedRoutes,
	components::{Outlet, ParentRoute},
	path,
};
use models::frontend::workspace::deployment::*;

use self::deployment::*;
use crate::prelude::*;

/// A holder component for all workspaced related routes
#[component]
pub fn WorkspacedHolder() -> impl IntoView {
	view! {
		<Outlet />
	}
}

/// The content to show when the user is logged in and has a workspace
#[component(transparent)]
pub fn WorkspacedContent() -> impl MatchNestedRoutes + Clone {
	view! {
		<ParentRoute path=path!("") view=Outlet>
			<AppRoute<ListDeploymentsRoute, _, _> view=ListDeploymentsPage />
			<AppRoute<CreateDeploymentRoute, _, _> view=CreateDeploymentPage />
			<AppRoute<DeploymentDetailsRoute, _, _> view=DeploymentDetailsPage />
		</ParentRoute>
	}
	.into_inner()
}
