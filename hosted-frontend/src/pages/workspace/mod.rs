use crate::prelude::*;

mod create;
mod manage_workspace;
mod sidebar;
mod tabs;

pub use self::{create::*, manage_workspace::*, sidebar::*, tabs::*};

#[component]
pub fn WorkspacePage() -> impl IntoView {
	view! {
		<ContainerMain class="w-full h-full my-md">
			<Outlet />
		</ContainerMain>
	}
}
