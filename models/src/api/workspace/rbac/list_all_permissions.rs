use crate::{prelude::*, utils::BearerToken};
use serde::{Deserialize, Serialize};

/// The permission metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Permission {
	/// The name of the permission
	pub name: String,
	/// The descripton of the permission
	pub description: String,
}

macros::declare_api_endpoint!(
	/// Route to list all the permissions
	ListAllPermissions,
	GET "/workspace/:workspace_id/rbac/permission" {
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
		/// The list permissions that contains:
		/// 	name - The name of the permission
		/// 	description - The description of the permission
		pub permissions: Vec<WithId<Permission>>
	}
);
