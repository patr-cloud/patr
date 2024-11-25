use crate::{pages::CreateWorkspace, prelude::*};

::macros::declare_app_route! {
	/// Route for Workspace Page
	CreateWorkspace,
	"/workspace/create",
	requires_login = true,
}

/// The list of routes for the workspace stuff
#[component(transparent)]
pub fn WorkspaceRoutes() -> impl IntoView {
	view! { <AppRoute<CreateWorkspaceRoute, _, _> view={move |_, _| CreateWorkspace} /> }
}
