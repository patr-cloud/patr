use crate::prelude::*;

macros::declare_api_endpoint!(
	/// The route to list all the available recovery options when user forgets their
	/// password and opt for changing it. The email address and phone number are
	/// masked for privacy reasons, so this route can be called without any authentication.
	ListRecoveryOptions,
	POST "/auth/list-recovery-options",
	request = {
		/// The user ID of the user
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
