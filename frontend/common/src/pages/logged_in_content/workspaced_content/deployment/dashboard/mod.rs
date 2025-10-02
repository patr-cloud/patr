/// The Deployment Dashboard Head
mod head;

use self::head::*;
use super::components::*;
use crate::prelude::*;

/// The Deployment Dashboard
#[component]
pub fn DeploymentDashboard() -> impl IntoView {
	let deployments = vec![
		"Deployment 1".to_owned(),
		"Deployment 2".to_owned(),
		"Deployment 3".to_owned(),
	];
	view! {
		// <DeploymentPage>
		// 	<DeploymentDashboardHead />
		// 	<ContainerGrid
		// 		min_width={"300px"}
		// 		max_width={"400px"}
		// 	>
		// 		<For
		// 			each={move || deployments.clone()}
		// 			key={|state| state.clone()}
		// 			let:_
		// 		>
		// 			<DeploymentCard />
		// 		</For>
		// 	</ContainerGrid>
		// </DeploymentPage>
	}
}
