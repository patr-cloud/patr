/// The Deployment Components, such as the Deployment Card, inputs, etc.
mod components;
/// The Deployment Dashboard Page
mod dashboard;

use models::frontend::workspace::deployment::{
	CreateDeploymentQuery,
	CreateDeploymentRoute,
	DeploymentDetailsQuery,
	DeploymentDetailsRoute,
	ListDeploymentsQuery,
	ListDeploymentsRoute,
};

pub use self::dashboard::*;
use crate::prelude::*;

/// Temporary Page Container
#[component]
pub fn TempPageContainer(children: Children) -> impl IntoView {
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
pub fn DeploymentPage(children: Children) -> impl IntoView {
	view! {
		<ContainerMain class="w-full h-full my-md">
			{children()}
		</ContainerMain>
	}
}

#[expect(non_snake_case)]
pub fn ListDeploymentsPage(
	ListDeploymentsQuery { workspace_id }: ListDeploymentsQuery,
	ListDeploymentsRoute {}: ListDeploymentsRoute,
) -> impl IntoView {
	view! {}
}

#[expect(non_snake_case)]
pub fn CreateDeploymentPage(
	CreateDeploymentQuery { workspace_id }: CreateDeploymentQuery,
	CreateDeploymentRoute {}: CreateDeploymentRoute,
) -> impl IntoView {
	view! {}
}

#[expect(non_snake_case)]
pub fn DeploymentDetailsPage(
	DeploymentDetailsQuery { workspace_id }: DeploymentDetailsQuery,
	DeploymentDetailsRoute { deployment_id }: DeploymentDetailsRoute,
) -> impl IntoView {
	view! {}
}
