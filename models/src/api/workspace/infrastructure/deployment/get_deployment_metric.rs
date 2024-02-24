use super::{DeploymentMetrics, Interval, Step};
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to get monitoring metrics like CPU, RAM and Disk usage
	/// for a deployment
	GetDeploymentMetric,
	GET "/workspace/:workspace_id/infrastructure/deployment/:deployment_id/metrics" {
		/// The workspace ID of the user
		pub workspace_id: Uuid,
		/// The deployment ID to get the metrics for
		pub deployment_id: Uuid,
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.deployment_id
		}
	},
	query = {
		/// The interval of the metric to fetch where start_time is the starting duration
		/// All metrics from the start_time to the current time will be fetched
		pub start_time: Option<Interval>,
		/// The set intervals like 1min, 5min, 10mins, etc
		pub step: Option<Step>
	},
	response = {
		/// The deployment metrics containing:
		/// pod_name - The name of the pod
		/// metrics - The metrics of the pod
		pub metrics: Vec<DeploymentMetrics>
	}
);
