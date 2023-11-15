use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Creates a new container repository in the workspace.
	CreateContainerRepository,
	POST "/workspace/:workspace_id/container-registry" {
		/// The workspace to create the container repository in.
		pub workspace_id: Uuid,
	},
	request_headers = {
		/// The authorization token
		pub authorization: BearerToken,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.workspace_id
		}
	},
	request = {
		/// The name of the repository to create.
		pub name: String,
	},
	response = {
		/// The id of the created repository.
		pub id: Uuid,
	}
);
