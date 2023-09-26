use crate::{prelude::*, utils::BearerToken};

macros::declare_api_endpoint!(
	/// Route to delete a workspace. Only the super admin of a workspace can delete a workspace
	DeleteWorkspace,
	DELETE "/workspace/:workspace_id" {
		/// The ID of the workspace to be deleted
		pub workspace_id: Uuid,
	},
	request_headers = {
		/// The authorization token
		pub authorization: BearerToken,
	},
	authentication = {
		AppAuthentication::<Self>::WorkspaceSuperAdminAuthenticator {
			extract_workspace_id: |req| req.path.workspace_id,
		}
	},
);
