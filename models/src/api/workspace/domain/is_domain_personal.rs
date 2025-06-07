use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to check if the domain is personal
	IsDomainPersonal,
	GET "/workspace/:workspace_id/is-domain-personal" {
		/// The ID of the workspace
		pub workspace_id: Uuid,
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	request = {
		/// The name of the domain
		#[preprocess(domain)]
		pub domain: String,
	},
	response = {
		/// Whether the domain is personal or not
		pub personal: bool,
		/// Whether the domain is being used by others
		pub is_used_by_others: bool,
	}
);
