use time::Duration;

use super::RunnerMetrics;
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to get system metrics (CPU, memory, disk, network) for a runner
	GetRunnerMetrics,
	GET "/workspace/{workspace_id}/runner/{runner_id}/metrics" {
		/// The workspace ID of the user
		pub workspace_id: Uuid,
		/// The runner ID to get the metrics for
		pub runner_id: Uuid,
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
		/// The runner system metrics, organized by metric type
		#[serde(flatten)]
		pub metrics: RunnerMetrics
	},
	audit_log = NoAuditLogger,
);
