use leptos_router::*;

use crate::prelude::*;

/// The Routes for the Infrastructure Pages,
/// Contains Secrets, Databases, Deployments, Static Sites, Repository, and
/// Container Registry
#[component(transparent)]
pub fn InfrastructureRoutes() -> impl IntoView {
	view! {
		<Route path={AppRoutes::Empty} view={|| view! { <Outlet /> }}>
			<Route path={LoggedInRoute::Secret} view={SecretsDashboard} />
			<Route path={LoggedInRoute::Database} view={DatabasePage}>
				<Route path="create" view={CreateDatabase} />
				<Route path="/:database_id" view={ManageDatabase} />
				<Route path={AppRoutes::Empty} view={DatabaseDashboard} />
			</Route>

			<Route path={LoggedInRoute::Deployment} view={DeploymentPage}>
				<Route path="create" view={CreateDeployment} />
				<Route path="/:deployment_id" view={ManageDeployments}>
					<Route path="/history" view={ManageDeploymentImageHistory} />
					<Route path="/logs" view={ManageDeploymentsLogs} />
					<Route path="/monitor" view={ManageDeploymentsMonitoring} />
					<Route path="/scaling" view={ManageDeploymentScaling} />
					<Route path="/urls" view={ManageDeploymentUrls} />
					<Route path={AppRoutes::Empty} view={ManageDeploymentDetailsTab} />
				</Route>
				<Route path={AppRoutes::Empty} view={DeploymentDashboard} />
			</Route>

			<Route path={LoggedInRoute::StaticSites} view={StaticSiteDashboard} />

			<Route path={LoggedInRoute::ContainerRegistry} view={ContainerRegistry}>
				<Route path="/:id" view={ManageRepository}>
					<Route path={AppRoutes::Empty} view={EditRepository} />
				</Route>
				<Route path="/new" view={CreateRepository} />
				<Route path={AppRoutes::Empty} view={ContainerRegistryDashboard} />
			</Route>
		</Route>
	}
}

mod container_registry;
mod database;
mod deployment;
mod secret;
mod static_sites;

pub use self::{container_registry::*, database::*, deployment::*, secret::*, static_sites::*};
