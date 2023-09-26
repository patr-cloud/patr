use crate::{prelude::*, utils::BearerToken};

macros::declare_api_endpoint!(
	/// Route to check if a workspace name is available
	IsWorkspaceNameAvailable,
	GET "/workspace/name-available",
	authentication = {
		AppAuthentication::<Self>::PlainTokenAuthenticator
	},
	request_headers = {
		/// The authorization token
		pub authorization: BearerToken,
	},
	query = {
		/// The name of the workspace to check
		pub name: String,
	},
	response = {
		/// Whether the workspace name is available
		pub available: bool,
	}
);
