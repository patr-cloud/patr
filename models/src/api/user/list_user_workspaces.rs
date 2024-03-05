use crate::{api::workspace::Workspace, prelude::*};

macros::declare_api_endpoint!(
	/// List all the workspaces that the currently authenticated user is a part of.
	ListUserWorkspaces,
	GET "/user/workspaces",
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
		/// The list of workspaces that the user is a part of.
		pub workspaces: Vec<WithId<Workspace>>,
	},
);
