use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to list all the available recovery options when user forgets their
	/// password and opt for changing it. The current recovery options are email and phone number.
	/// The backend performs validation and prevents the leak of sensitive user information.
	ListRecoveryOptions,
	GET "/auth/list-recovery-options",
	api = false,
	request_headers = {
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	request = {
		/// The user identifier of the user
		/// It can be either the username or the email of the user depending on the user input
		#[preprocess(trim, length(min = 2), regex = r"^[a-z0-9_][a-z0-9_\.\-]*[a-z0-9_]$")]
		pub user_id: String,
	},
	response = {
		/// The available phone number the user has linked to their account
		#[serde(skip_serializing_if = "Option::is_none")]
		pub recovery_phone_number: Option<String>,
		/// The available email the user has linked to their account
		#[serde(skip_serializing_if = "Option::is_none")]
		pub recovery_email: Option<String>,
	}
);
