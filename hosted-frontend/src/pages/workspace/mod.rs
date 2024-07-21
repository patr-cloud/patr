use crate::prelude::*;

mod create;
mod manage_workspace;
mod tabs;

pub use self::{create::*, manage_workspace::*, tabs::*};

#[component(transparent)]
pub fn WorkspaceRoutes() -> impl IntoView {
	view! {
		<Route ssr=SsrMode::InOrder path={AppRoutes::Empty} view={|| view! { <Outlet /> }}>
			<Route path={LoggedInRoute::Workspace} view={WorkspacePage}>
				<Route path={AppRoutes::Empty} view={ManageWorkspace}>
					<Route path="/list" view={ListWorksapce} />
					<Route path={AppRoutes::Empty} view={ManageWorkspaceSettingsTab} />
				</Route>
				<Route path="/create" view={CreateWorkspace} />
			</Route>
		</Route>
	}
}

#[component]
pub fn WorkspacePage() -> impl IntoView {
	view! {
		<ContainerMain class="full-width full-height mb-md">
			<Outlet />
		</ContainerMain>
	}
}
