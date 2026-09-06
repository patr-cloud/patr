use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to preview a workspace invite before accepting it. Returns the name
	/// of the workspace the invite is for, so the accept page can ask the user
	/// to confirm ("You've been invited to join {workspace}"). Does not consume
	/// the invite.
	PreviewWorkspaceInvite,
	POST "/user/workspace-invite/preview",
	request_headers = {
		/// The authorization token
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::PlainTokenAuthenticator
	},
	client_type = [WebDashboard],
	request = {
		/// The ID of the invite to preview
		#[preprocess(none)]
		pub invite_id: Uuid,
		/// The invite token from the email link
		#[preprocess(trim)]
		pub token: String,
	},
	response = {
		/// The name of the workspace the invite is for
		pub workspace_name: String,
	},
	audit_log = NoAuditLogger,
);
