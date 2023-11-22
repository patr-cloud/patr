use crate::{prelude::*, utils::BearerToken};
use serde::{Deserialize, Serialize};

/// The Resource Type metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
	GET "/workspace/:workspace_id/rbac/resource-type" {
		/// The ID of the workspace
		pub workspace_id: Uuid
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.workspace_id
		}
	},
	response = {
		/// The list of resource type containing:
		/// 	name - The name of the resource type
		/// 	description - The description of the resource type
		pub resource_types: Vec<WithId<ResourceType>>,
	}
);
