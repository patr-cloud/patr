use time::OffsetDateTime;

use super::RunnerLog;
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to get the logs of a runner process
	GetRunnerLogs,
	GET "/workspace/{workspace_id}/runner/{runner_id}/logs" {
		/// The workspace ID of the user
		pub workspace_id: Uuid,
		/// The runner ID to get the logs for
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
		/// The time up until which the runner logs should be fetched
		#[ts(type = "Date")]
		pub end_time: Option<OffsetDateTime>,
		/// The limit of logs to fetch. Defaults to 100
		#[preprocess(range(max = Some(500)))]
		pub limit: Option<u32>,
		/// The search query to filter logs
		pub search: Option<String>,
	},
	response = {
		/// The runner logs
		pub logs: Vec<RunnerLog>
	},
	audit_log = NoAuditLogger,
);
