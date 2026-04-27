use crate::{prelude::*, utils::constants::RESOURCE_NAME_REGEX};

macros::declare_api_endpoint!(
	/// Approve a runner link. Called from the browser by the logged-in user
	/// after reviewing the metadata on the consent page. In one transaction:
	/// creates a per-runner role with the runner-permission bundle, creates
	/// a service account assigned to that role, inserts the runner row with
	/// `service_account_id` set, and writes the SA token + runner ID into
	/// the Redis link entry so the CLI's next verify poll sees `Approved`.
	///
	/// The `workspace_id` here is the workspace currently selected in the
	/// app context — the consent UI doesn't render a dropdown but does send
	/// the value.
	ApproveRunnerLink,
	POST "/workspace/{workspace_id}/runner/link/{user_code}/approve" {
		/// Workspace to add the runner to (taken from the user's current
		/// workspace context in the UI, validated against their permissions
		/// here).
		pub workspace_id: Uuid,
		/// The user-typeable code from the verification URL.
		pub user_code: String,
	},
	request_headers = {
		/// Bearer token for the logged-in user.
		pub authorization: BearerToken,
		/// The user-agent of the browser.
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.workspace_id,
			extract_workspace_id: |req| req.path.workspace_id,
			permission: Permission::Runner(RunnerPermission::Create),
		}
	},
	request = {
		/// Display name for the new runner.
		#[preprocess(trim, regex = RESOURCE_NAME_REGEX)]
		pub runner_name: String,
	},
	client_type = [WebDashboard],
	audit_log = AppAuditLogger {
		audit_log_type: AuditLogType::ResourceCreated,
		resource_type: ResourceType::Runner,
		extract_resource_id: ResourceIdExtractor::FromRequest(|req| req.path.workspace_id),
	},
);
