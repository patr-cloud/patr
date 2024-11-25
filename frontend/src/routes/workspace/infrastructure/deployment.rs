use models::{api::workspace::deployment::DeploymentStatus, prelude::*};
use serde::{Deserialize, Serialize};

use crate::{
	pages::{
		CreateDeployment,
		DeploymentDashboard,
		DeploymentPage,
		ManageDeploymentDetailsTab,
		ManageDeploymentImageHistory,
		ManageDeploymentScaling,
		ManageDeploymentUrls,
		ManageDeployments,
		ManageDeploymentsLogs,
		ManageDeploymentsMonitoring,
	},
	prelude::*,
	CommonQueryParams,
};

/// Columns that can be filtered by,
#[derive(
	Debug, Clone, Serialize, Deserialize, PartialEq, Eq, strum::Display, strum::EnumString,
)]
#[strum(serialize_all = "camelCase")]
pub enum FilterableColumns {
	/// Filter by the name of the deployment
	Name,
	/// Filter by the runner name of the deployment
	RunnerName,

	/// Filter by the status of the deployment
	Status,
	/// Filter by the time the deployment was created
	Created,
	/// Filter by the time the deployment was last updated
	LastUpdated,
	/// Filter by the image name of the deployment
	ImageName,
	/// Filter by the image tag of the deployment
	ImageTag,
}

/// Sortable columns for the deployments
#[derive(
	Debug, Clone, Serialize, Deserialize, PartialEq, Eq, strum::Display, strum::EnumString,
)]
#[strum(serialize_all = "camelCase")]
pub enum SortableColumns {
	/// Sort by the name of the deployment
	Name,
	/// Sort by the runner name of the deployment
	RunnerName,
	/// Sort by the created time of the deployment
	Created,
	/// Sort by the last updated time of the deployment
	LastUpdated,
	/// Sort by the status of the deployment
	Status,
}

::macros::declare_app_route! {
	/// Route for Deployments Dashboard Page
	DeploymentsDashboard,
	"/deployment",
	requires_login = true,
	query = {
		/// Common Query Parameters
		#[serde(flatten)]
		pub common: CommonQueryParams,
		/// The Column to sort by
		#[serde(skip_serializing_if = "Option::is_none")]
		pub sort: Option<SortableColumns>,
		/// Filter the deployments by the status
		#[serde(skip_serializing_if = "Option::is_none")]
		pub status: Option<OneOrMore<DeploymentStatus>>,
		/// The Column to filter query by
		#[serde(skip_serializing_if = "Option::is_none")]
		pub filter_by: Option<FilterableColumns>,
	}
}

::macros::declare_app_route! {
	/// Route for Deployments Create Page
	CreateDeployment,
	"/deployment/create",
	requires_login = true,
}

::macros::declare_app_route! {
	/// Route for Deployments Details Page
	ManageDeployment,
	"/deployment/:deployment_id" {
		/// The id of the deployment
		pub deployment_id: Uuid,
	},
	requires_login = true,
}

::macros::declare_app_route! {
	/// Route for Deployments History Page
	ManageDeploymentImageHistory,
	"/deployment/:deployment_id/history" {
		/// The id of the deployment
		pub deployment_id: Uuid,
	},
	requires_login = true,
	query = {
		/// Common Query Parameters
		#[serde(flatten)]
		pub common: CommonQueryParams,
	}
}

::macros::declare_app_route! {
	/// Route for Deployments Logs Page
	ManageDeploymentsLogs,
	"/deployment/:deployment_id/logs" {
		/// The id of the deployment
		pub deployment_id: Uuid,
	},
	requires_login = true,
	query = {
		/// Common Query Parameters
		#[serde(flatten)]
		pub common: CommonQueryParams,
	}
}

::macros::declare_app_route! {
	/// Route for Deployments Monitoring Page
	ManageDeploymentsMonitoring,
	"/deployment/:deployment_id/monitor" {
		/// The id of the deployment
		pub deployment_id: Uuid,
	},
	requires_login = true,
}

::macros::declare_app_route! {
	/// Route for Deployments Scaling Page
	ManageDeploymentScaling,
	"/deployment/:deployment_id/scaling" {
		/// The id of the deployment
		pub deployment_id: Uuid,
	},
	requires_login = true,
}

::macros::declare_app_route! {
	/// Route for Deployments URLs Page
	ManageDeploymentUrls,
	"/deployment/:deployment_id/urls" {
		/// The id of the deployment
		pub deployment_id: Uuid,
	},
	requires_login = true,
}

/// The routes for the deployment pages
#[component(transparent)]
pub fn DeploymentRoutes() -> impl IntoView {
	view! {
		<AppRoute<DeploymentsDashboardRoute, _, _> view={move |_query, _params| DeploymentPage}>
			<AppRoute<CreateDeploymentRoute, _, _> view={move |_query, _params| CreateDeployment} />
			<AppRoute<ManageDeploymentRoute, _, _> view={move |_query, _params| ManageDeployments}>
				<AppRoute<ManageDeploymentUrlsRoute, _, _,>
					view={move |_query, _params| ManageDeploymentUrls}
				/>
				<AppRoute<ManageDeploymentImageHistoryRoute, _, _,>
					view={move |_query, _params| ManageDeploymentImageHistory}
				/>
				<AppRoute<ManageDeploymentsLogsRoute, _, _,>
					view={move |_query, _params| ManageDeploymentsLogs}
				/>
				<AppRoute<ManageDeploymentsMonitoringRoute, _, _,>
					view={move |_query, _params| ManageDeploymentsMonitoring}
				/>
				<AppRoute<ManageDeploymentScalingRoute, _, _,>
					view={move |_query, _params| ManageDeploymentScaling}
				/>
				<Route path={AppRoutes::Empty} view={ManageDeploymentDetailsTab} />
			</AppRoute<ManageDeploymentRoute, _, _>>
			<Route path={AppRoutes::Empty} view={DeploymentDashboard} />
		</AppRoute<DeploymentsDashboardRoute, _, _>>
	}
}
