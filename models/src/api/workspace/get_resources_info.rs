use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{prelude::*, rbac::ResourceType};

/// Metadata about a single resource, resolved from its ID. An ID that does not
/// correspond to a live resource in this workspace (deleted, or owned by
/// another workspace) is represented by a `None` entry in the response list
/// rather than by null fields here, so a `ResourceInfo` always carries a real
/// `resource_type`. `name` stays optional because some resource types are
/// genuinely nameless (e.g. a managed URL has no name column).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "camelCase")]
pub struct ResourceInfo {
	/// The human-readable name of the resource. `None` when the resource type
	/// has no name (e.g. a managed URL).
	pub name: Option<String>,
	/// The type of the resource.
	pub resource_type: ResourceType,
}

macros::declare_api_endpoint!(
	/// Route to resolve a batch of resource IDs into their names and types. This
	/// is used, for example, to display the resources a role's permissions apply
	/// to, where only the resource IDs are stored.
	GetResourcesInfo,
	POST "/workspace/{workspace_id}/resources-info" {
		/// The ID of the workspace
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
	request = {
		/// The set of resource IDs to resolve.
		#[preprocess(none)]
		pub resource_ids: BTreeSet<Uuid>,
	},
	response = {
		/// The resolved resources, one entry per requested ID, with `null` for
		/// any ID that could not be resolved in this workspace.
		pub resources: Vec<Option<WithId<ResourceInfo>>>,
	},
	client_type = [ApiToken, ServiceAccount, WebDashboard],
	audit_log = NoAuditLogger,
);
