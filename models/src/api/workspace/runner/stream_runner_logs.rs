use time::OffsetDateTime;

use super::RunnerLog;
use crate::prelude::*;

macros::declare_stream_endpoint!(
	/// Route to stream the logs of a runner process in real time
	StreamRunnerLogs,
	GET "/workspace/{workspace_id}/runner/{runner_id}/logs/stream" {
		/// The workspace ID of the user
		pub workspace_id: Uuid,
		/// The runner ID to stream the logs for
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
		/// The time from which the runner logs should be streamed
		pub start_time: Option<OffsetDateTime>,
	},
	server_msg = {
		/// There is new log data for the runner
		LogData {
			/// The log entries
			logs: Vec<RunnerLog>,
		},
	},
	client_msg = {},
	audit_log = NoAuditLogger,
);
