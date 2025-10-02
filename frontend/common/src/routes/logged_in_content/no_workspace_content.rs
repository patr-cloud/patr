use leptos_router::{
	MatchNestedRoutes,
	components::{Outlet, ParentRoute},
	path,
};
use models::frontend::workspace::*;

use crate::prelude::*;

/// A holder component for all no-workspace related routes
#[component]
pub fn NoWorkspaceHolder() -> impl IntoView {
	view! {
		<Outlet/>
	}
}

/// The content to show when the user is logged in but has no workspace
#[component(transparent)]
pub fn NoWorkspaceContent() -> impl MatchNestedRoutes + Clone {
	view! {
		<ParentRoute path=path!("") view=Outlet>
			// TODO: Show create workspace page when ready
			<AppRoute<CreateWorkspaceRoute, _, _> view=CreateWorkspacePage />
		</ParentRoute>
	}
	.into_inner()
}
