use crate::{prelude::*, utils::constants::USER_NAME_REGEX};

macros::declare_api_endpoint!(
	/// Set the information of the currently authenticated user.
	UpdateUserInfo,
	PATCH "/user",
	client_type = [WebDashboard],
	request_headers = {
		/// The authorization token
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::PlainTokenAuthenticator
	},
	request = {
		/// The first name of the user.
		#[preprocess(trim, regex = USER_NAME_REGEX)]
		pub first_name: String,
		/// The last name of the user.
		#[preprocess(trim, regex = USER_NAME_REGEX)]
		pub last_name: String,
	},
	audit_log = NoAuditLogger,
);
