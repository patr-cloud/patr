use time::Duration;

use super::RunnerMetricName;
use crate::{api::workspace::MetricDataPoint, prelude::*};

macros::declare_api_endpoint!(
	/// Route to get a single system metric for a runner. The metric name is
	/// specified in the path (e.g. `system_cpu_usage`, `system_network_rx`).
	GetRunnerMetrics,
	GET "/workspace/{workspace_id}/runner/{runner_id}/metrics/{metric}" {
		/// The workspace ID of the user
		pub workspace_id: Uuid,
		/// The runner ID to get the metrics for
		pub runner_id: Uuid,
		/// The metric to query
		pub metric: RunnerMetricName,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.runner_id,
			extract_workspace_id: |req| req.path.workspace_id,
			permission: Permission::Runner(RunnerPermission::View),
		}
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	query = {
		/// The duration for which the runner metrics are fetched. Max 30 days.
		#[preprocess(range(max = Some(Duration::days(30))))]
		#[ts(type = "Number")]
		pub interval: Option<Duration>,
	},
	response = {
		/// The metric data points
		pub data_points: Vec<MetricDataPoint>
	},
	client_type = [ApiToken, ServiceAccount, WebDashboard],
	audit_log = NoAuditLogger,
);
