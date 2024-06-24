use crate::{prelude::*, utils::constants::RESOURCE_NAME_REGEX};

macros::declare_api_endpoint!(
	/// Route to create a new workspace. The user that called this route will automatically be assigned as the super admin of the workspace.
	CreateWorkspace,
	POST "/workspace",
	request = {
		/// The name of the workspace to be created
		#[preprocess(trim, regex = RESOURCE_NAME_REGEX)]
		pub name: String,
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
		#[serde(flatten)]
		pub id: WithId<()>,
	},
);
