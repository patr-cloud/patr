use crate::{prelude::*, utils::BearerToken};

use super::Secret;

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
	pagination = true,
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.workspace_id
		}
	},
	response_headers = {
		/// The total number of items in the pagination
		pub total_count: TotalCountHeader,
	},
	response = {
		/// The list of secrets that contains:
		///     name - The secret name
		///     deployment_id - The deployment this secret is attached to
		#[serde(flatten)]
		pub secrets: Vec<WithId<Secret>>
	}
);
