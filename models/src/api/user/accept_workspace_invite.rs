use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route for the currently authenticated user to accept a workspace invite.
	/// The `invite_id` and `token` come from the invite email link. The caller
	/// must own the email address the invite was sent to.
	AcceptWorkspaceInvite,
	POST "/user/workspace-invite/accept",
	request_headers = {
		/// The authorization token
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::PlainTokenAuthenticator
	},
	api = false,
	request = {
		/// The ID of the invite being accepted
		#[preprocess(none)]
		pub invite_id: Uuid,
		/// The invite token from the email link
		#[preprocess(trim)]
		pub token: String,
	},
	response = {
		/// The ID of the workspace the user just joined
		#[serde(flatten)]
		pub id: OnlyId,
	},
	audit_log = NoAuditLogger,
);
