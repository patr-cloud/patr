use crate::prelude::*;

/// The Entry Point for the whole app, here's where routers and all are defined
#[component]
pub fn App() -> impl IntoView {
	view! {
		<TempPageContainer>
			<DeploymentDashboard />
		</TempPageContainer>
	}
}
