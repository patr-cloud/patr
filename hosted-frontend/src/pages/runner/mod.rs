mod create_runner;
mod dashboard;
mod manage_runner;

use leptos_router::*;

pub use self::{create_runner::*, dashboard::*, manage_runner::*};
use crate::prelude::*;

#[component(transparent)]
pub fn RunnerRoutes() -> impl IntoView {
	view! {
		<Route path={LoggedInRoute::Runners} view={RunnerPage}>
			<Route path={"create"} view={CreateRunner}/>
			<Route path={":runner_id"} view={ManageRunner}/>
			<Route path={AppRoutes::Empty} view={RunnerDashboard}/>
		</Route>
	}
}

#[component]
pub fn RunnerPage() -> impl IntoView {
	view! {
		<ContainerMain>
			<Outlet />
		</ContainerMain>
	}
}
