/// The Deployment Components, such as the Deployment Card, inputs, etc.
mod components;
/// The Deployment Dashboard Page
mod dashboard;

pub use self::dashboard::*;
use crate::prelude::*;

/// Temporary Page Container
#[component]
pub fn TempPageContainer(children: Children) -> Element {
	view! {
		<div class="font-primary flex items-start justify-start w-full h-full bg-secondary {}">
			<aside class="sidebar flex flex-col items-start justify-start pb-xl">
				<div></div>
			</aside>
			<main class="flex flex-col w-full px-lg min-h-screen">
				{children()}
			</main>
		</div>
	}
}

/// The Outer Shell for All Deployment Pages
#[component]
pub fn DeploymentPage(children: Children) -> Element {
	view! {
		<ContainerMain class="w-full h-full my-md">
			{children()}
		</ContainerMain>
	}
}
