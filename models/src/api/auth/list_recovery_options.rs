macros::declare_api_endpoint!(
	/// Definition of a route to list all the available recovery options when user forgets their 
	/// password and opt for changing it. The current recovery options are email and phone number.
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
