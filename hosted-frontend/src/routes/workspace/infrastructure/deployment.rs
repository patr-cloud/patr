use models::{api::workspace::deployment::DeploymentStatus, prelude::*};
use serde::{Deserialize, Serialize};

use crate::CommonQueryParams;

#[derive(
	Debug, Clone, Serialize, Deserialize, PartialEq, Eq, strum::Display, strum::EnumString,
)]
#[strum(serialize_all = "camelCase")]
pub enum FilterableColumns {
	Name,
	RunnerName,
	Status,
	Created,
	LastUpdated,
	ImageName,
	ImageTag,
}

#[derive(
	Debug, Clone, Serialize, Deserialize, PartialEq, Eq, strum::Display, strum::EnumString,
)]
#[strum(serialize_all = "camelCase")]
pub enum SortableColumns {
	Name,
	RunnerName,
	Created,
	LastUpdated,
	Status,
}

macros::declare_app_route! {
	/// Route for Deployments Dashboard Page
	ListDeployments,
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

macros::declare_app_route! {
	/// Route for Deployments Create Page
	CreateDeployment,
	"/deployment/create",
	requires_login = true,
	query = { }
}

macros::declare_app_route! {
	/// Route for Deployments Details Page
	ManageDeploymentDetailsTab,
	"/deployment/:deployment_id" {
		/// The id of the deployment
		pub deployment_id: Uuid,
	},
	requires_login = true,
	query = {}
}

macros::declare_app_route! {
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

macros::declare_app_route! {
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

macros::declare_app_route! {
	/// Route for Deployments Monitoring Page
	ManageDeploymentsMonitoring,
	"/deployment/:deployment_id/monitor" {
		/// The id of the deployment
		pub deployment_id: Uuid,
	},
	requires_login = true,
	query = {}
}

macros::declare_app_route! {
	/// Route for Deployments Scaling Page
	ManageDeploymentScaling,
	"/deployment/:deployment_id/scaling" {
		/// The id of the deployment
		pub deployment_id: Uuid,
	},
	requires_login = true,
	query = {}
}

macros::declare_app_route! {
	/// Route for Deployments URLs Page
	ManageDeploymentUrls,
	"/deployment/:deployment_id/urls" {
		/// The id of the deployment
		pub deployment_id: Uuid,
	},
	requires_login = true,
	query = {}
}
