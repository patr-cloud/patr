/// The Deployment Dashboard Head
mod head;

use models::frontend::workspace::deployment::{ListDeploymentsQuery, ListDeploymentsRoute};

use self::head::*;
use super::components::*;
use crate::prelude::*;

/// The Deployment Dashboard
#[expect(non_snake_case)]
pub fn DeploymentDashboard(
	ListDeploymentsQuery { workspace_id }: ListDeploymentsQuery,
	ListDeploymentsRoute {}: ListDeploymentsRoute,
) -> impl IntoView {
	let deployments = vec![
		"Deployment 1".to_owned(),
		"Deployment 2".to_owned(),
		"Deployment 3".to_owned(),
	];

	view! {
		<DeploymentPage>
			<DeploymentDashboardHead />
			<ContainerGrid
				min_width={"300px"}
				max_width={"400px"}
			>
				<For
					each={move || deployments.clone()}
					key={|state| state.clone()}
					let:_
				>
					<DeploymentCard />
				</For>
			</ContainerGrid>
		</DeploymentPage>
	}
}
