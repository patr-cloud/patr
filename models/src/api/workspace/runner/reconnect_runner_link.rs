use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Reconnect an existing runner to a new machine. Called from the browser by
	/// the logged-in user after picking one of the workspace's runners on the
	/// consent page. Rotates the runner's service-account token — the old token
	/// is immediately invalidated — and writes the new token + runner ID into
	/// the Redis link entry so the CLI's next verify poll sees `Approved`.
	///
	/// Unlike [`super::ApproveRunnerLink`] this creates nothing: the runner row,
	/// its per-runner role, its service account, and its Cloudflare tunnel are
	/// all preserved. Only the SA token is rotated, so the runner keeps its
	/// identity and its deployment associations.
	ReconnectRunnerLink,
	POST "/workspace/{workspace_id}/runner/link/{user_code}/reconnect/{runner_id}" {
		/// Workspace the runner belongs to (from the user's current workspace
		/// context in the UI, validated against their permissions here).
		pub workspace_id: Uuid,
		/// The user-typeable code from the verification URL.
		pub user_code: String,
		/// The runner to reconnect. Its service-account token is rotated.
		pub runner_id: Uuid,
	},
	request_headers = {
		/// Bearer token for the logged-in user.
		pub authorization: BearerToken,
		/// The user-agent of the browser.
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.runner_id,
			extract_workspace_id: |req| req.path.workspace_id,
			permission: Permission::Runner(RunnerPermission::RegenerateToken),
		}
	},
	client_type = [WebDashboard],
	audit_log = AppAuditLogger {
		audit_log_type: AuditLogType::ResourceUpdated,
		resource_type: ResourceType::Runner,
		extract_resource_id: ResourceIdExtractor::FromRequest(|req| req.path.runner_id),
	},
);
