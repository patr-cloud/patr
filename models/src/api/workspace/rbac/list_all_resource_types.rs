use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::prelude::*;

/// The Resource Type metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "camelCase")]
pub struct ResourceType {
	/// The name of the resource type
	pub name: String,
	/// The description of the resource type
	pub description: String,
}

macros::declare_api_endpoint!(
	/// Route to list all resource types
	ListAllResourceTypes,
	GET "/rbac/resource-type",
	workspaced = true,
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
		/// The list of resource type containing:
		/// - name - The name of the resource type
		/// - description - The description of the resource type
		pub resource_types: Vec<WithId<ResourceType>>,
	},
	audit_log = NoAuditLogger,
);
