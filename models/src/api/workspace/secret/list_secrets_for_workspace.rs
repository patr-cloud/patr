use crate::{prelude::*, utils::BearerToken};
use serde::{Deserialize, Serialize};

/// Patr secrets which only contains the secret name and not the 
/// secret value. This is to ensure that Patr does not have
/// access to any user sensitive information.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Secret {
	/// The name of the secret
	pub name: String,
	/// The deployment the secret is attached to
	#[serde(skip_serializing_if = "Option::is_none")]
	pub deployment_id: Option<Uuid>,
}

macros::declare_api_endpoint!(
	/// Route to list all the secrets in a workspace
	ListSecretsForWorkspace,
	GET "/workspace/:workspace_id/secret" {
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
		/// The list of secrets that contains:
		///     name - The secret name
		///     deployment_id - The deployment this secret is attached to
		#[serde(flatten)]
		pub secrets: Vec<WithId<Secret>>
	}
);
