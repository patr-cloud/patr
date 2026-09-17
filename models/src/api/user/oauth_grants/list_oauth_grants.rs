use super::UserOAuthGrant;
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// List every app currently authorized to act on the user's behalf.
	///
	/// One row per grant, not per app: approving the same app's consent screen
	/// twice creates two, and the user should see both rather than have one
	/// silently stand in for the other.
	ListOAuthGrants,
	GET "/user/oauth-grant",
	api = false,
	request_headers = {
		/// The authorization token
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::PlainTokenAuthenticator
	},
	listable_resource = UserOAuthGrant,
	response_headers = {
		/// The total number of grants the user holds
		pub total_count: TotalCountHeader,
	},
	response = {
		/// The list of grants
		pub grants: Vec<WithId<UserOAuthGrant>>,
	},
	audit_log = NoAuditLogger,
);
