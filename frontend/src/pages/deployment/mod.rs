mod dashboard;

pub use self::dashboard::*;
use crate::prelude::*;

/// Temporary Page Container
#[component]
pub fn TempPageContainer(children: Children) -> impl IntoView {
	view! {
		<main class="flex items-start justify-start w-full h-full bg-secondary">
			<aside class="sidebar flex flex-col items-start justify-start pb-xl">
				<div></div>
			</aside>
			{children()}
		</main>
	}
}

/// The Outer Shell for All Deployment Pages
#[component]
pub fn DeploymentPage(children: Children) -> impl IntoView {
	view! {
		<ContainerMain class="w-full h-full my-md">
			{children()}
		</ContainerMain>
	}
}
