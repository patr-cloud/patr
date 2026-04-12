use time::Duration;

use super::DeploymentMetricName;
use crate::{api::workspace::MetricDataPoint, prelude::*};

macros::declare_api_endpoint!(
	/// Route to get a single metric for a deployment. The metric name is
	/// specified in the path (e.g. `ingress_rps`, `container_cpu_usage`).
	GetDeploymentMetric,
	GET "/workspace/{workspace_id}/deployment/{deployment_id}/metrics/{metric}" {
		/// The workspace ID of the user
		pub workspace_id: Uuid,
		/// The deployment ID to get the metrics for
		pub deployment_id: Uuid,
		/// The metric to query
		pub metric: DeploymentMetricName,
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.deployment_id,
			extract_workspace_id: |req| req.path.workspace_id,
			permission: Permission::Deployment(DeploymentPermission::View),
		}
	},
	query = {
		/// The duration for when the deployment metrics are fetched
		#[preprocess(range(max = Some(Duration::days(14))))]
		#[ts(type = "Number")]
		pub interval: Option<Duration>,
	},
	response = {
		/// The metric data points
		pub data_points: Vec<MetricDataPoint>
	},
	audit_log = NoAuditLogger,
);
