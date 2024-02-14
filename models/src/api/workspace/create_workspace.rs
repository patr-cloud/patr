use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to create a new workspace. The user that called this route will automatically be assigned as the super admin of the workspace.
	CreateWorkspace,
	POST "/workspace",
	request = {
		/// The name of the workspace to be created
		pub workspace_name: String,
	},
	request_headers = {
		/// The authorization token
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::PlainTokenAuthenticator
	},
	response = {
		/// The ID of the newly created workspace
		pub workspace_id: Uuid,
	},
);
