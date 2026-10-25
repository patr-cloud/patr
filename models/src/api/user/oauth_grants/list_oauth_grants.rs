use super::UserOAuthGrant;
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// List the apps acting on the current user's behalf, one row per grant.
	ListOAuthGrants,
	GET "/user/oauth-grant",
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
	client_type = [WebLogin],
	audit_log = NoAuditLogger,
);
