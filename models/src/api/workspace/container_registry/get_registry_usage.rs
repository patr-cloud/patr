use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to get the registry-wide usage summary for a workspace: how much
	/// storage all of its repositories take up (deduplicated across shared
	/// blobs), and how many repositories and images it holds.
	GetContainerRegistryUsage,
	GET "/workspace/{workspace_id}/container-registry/usage" {
		/// The workspace ID to get the registry usage of
		pub workspace_id: Uuid
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::WorkspaceMembershipAuthenticator {
			extract_workspace_id: |req| req.path.workspace_id
		}
	},
	response = {
		/// The total storage used by all repositories in the workspace, in
		/// bytes, with blobs shared across images counted only once.
		pub used_bytes: u64,
		/// The number of (non-deleted) repositories in the workspace.
		pub repository_count: u64,
		/// The number of image manifests across all repositories.
		pub image_count: u64,
	},
	audit_log = NoAuditLogger,
);
