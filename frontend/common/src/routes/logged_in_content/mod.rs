use leptos_router::{
	MatchNestedRoutes,
	components::{Outlet, ParentRoute, ProtectedParentRoute},
	path,
};
use models::frontend::workspace::{deployment::ListDeploymentsRoute, *};

use crate::prelude::*;

/// All pages that are shown when the user has no workspace at all
mod no_workspace_content;
/// All pages that are shown in the context of a workspace
mod workspaced_content;

use self::{no_workspace_content::*, workspaced_content::*};

/// The parent component for ALL logged in routes. All logged in content will be
/// nested inside this component
#[component]
pub fn LoggedInHolder() -> impl IntoView {
	view! {
		<Outlet/>
	}
}

/// The content to show when the user is logged in
#[component(transparent)]
pub fn LoggedInContent() -> impl MatchNestedRoutes + Clone {
	let has_workspaces = false; // TODO: Fetch from context when ready

	view! {
		<ParentRoute path=path!("") view=Outlet>
			<ProtectedParentRoute
				path=path!("")
				view=NoWorkspaceHolder
				condition=move || Some(!has_workspaces)
				redirect_path=|| "/"
			>
				<NoWorkspaceContent />
			</ProtectedParentRoute>
			<ProtectedParentRoute
				path=path!("")
				view=WorkspacedHolder
				condition=move || Some(has_workspaces)
				redirect_path=|| "/"
			>
				<WorkspacedContent />
			</ProtectedParentRoute>
		</ParentRoute>
	}
	.into_inner()
}
