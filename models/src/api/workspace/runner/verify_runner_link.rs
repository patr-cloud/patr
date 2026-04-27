use serde::{Deserialize, Serialize};

use crate::prelude::*;

/// Result of a verify poll. `Pending` means the user hasn't approved yet
/// (CLI keeps polling). `Approved` carries the issued service account token
/// and the runner+workspace it was attached to.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ts_rs::TS)]
#[serde(tag = "status", rename_all = "camelCase")]
#[ts(export)]
pub enum VerifyRunnerLinkResult {
	/// User hasn't approved yet. CLI should keep polling.
	Pending,
	/// User approved. The link entry is consumed; this response is one-shot.
	#[serde(rename_all = "camelCase")]
	Approved {
		/// ID of the runner that was created.
		runner_id: Uuid,
		/// Workspace the runner was added to.
		workspace_id: Uuid,
		/// Service account token (`patrv1.{refresh_token}.{sa_id}`) the
		/// runner uses to authenticate.
		token: String,
	},
}

macros::declare_api_endpoint!(
	/// Poll for approval of a runner link. Called by the CLI on a fixed
	/// interval after `POST /workspace/{workspace_id}/runner/link`. Returns
	/// `Pending` until the user approves; once approved, returns the runner
	/// ID + SA token in a one-shot response (the link entry is deleted on
	/// first successful claim).
	VerifyRunnerLink,
	POST "/workspace/{workspace_id}/runner/link/verify" {
		/// Workspace the link was created in.
		pub workspace_id: Uuid,
	},
	request_headers = {
		/// Bearer token for the logged-in user driving the CLI.
		pub authorization: BearerToken,
		/// The user-agent of the CLI making the request.
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
		/// User code returned by `POST /workspace/{workspace_id}/runner/link`.
		pub user_code: String,
		/// Device code returned by `POST /workspace/{workspace_id}/runner/link`.
		/// The server constant-time compares this against the stored value
		/// before returning credentials.
		pub device_code: String,
	},
	response = {
		/// Flattened so the wire shape is `{ "status": "pending" }` or
		/// `{ "status": "approved", "runnerId": ..., "workspaceId": ...,
		/// "token": ... }` instead of nesting under a `result` key.
		#[serde(flatten)]
		pub result: VerifyRunnerLinkResult,
	},
	client_type = [ApiToken],
	audit_log = NoAuditLogger,
);
