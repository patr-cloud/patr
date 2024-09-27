mod create_runner;
mod dashboard;
mod manage_runner;

use leptos_router::*;

pub use self::{create_runner::*, dashboard::*, manage_runner::*};
use crate::prelude::*;

/// The Wrapper Page for all Runner Pages
#[component]
pub fn RunnerPage() -> impl IntoView {
	view! {
		<ContainerMain class="my-md">
			<Outlet />
		</ContainerMain>
	}
}
