use crate::{pages::*, prelude::*};

/// Not Workspaced Routes, For example Profile or Workspace Routes.
/// Ideally used only when there is not a single workspace present
#[component(transparent)]
pub fn NotWorkspacedRoutes() -> impl IntoView {
	view! {
		<Route
			path={AppRoutes::Empty}
			view={|| {
				view! {
					<Sidebar sidebar_items={get_sidebar_items(expect_context::<AppType>())}>
						<div />
					</Sidebar>

					<main class="fc-fs-ct full-width px-lg">
						<Outlet />
					</main>
				}
			}}
		>
			<ProfileRoutes />
			<WorkspaceRoutes />
			<Route path="" view={|| view! { <div></div> }} />
		</Route>
	}
}
