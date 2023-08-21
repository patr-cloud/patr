macros::declare_api_endpoint!(
	ListRecoveryOptions,
	POST "/auth/list-recovery-options",
	request = {
		pub user_id: String,
	},
	response = {
		#[serde(skip_serializing_if = "Option::is_none")]
		pub recovery_phone_number: Option<String>,
		#[serde(skip_serializing_if = "Option::is_none")]
		pub recovery_email: Option<String>,
	}
);
