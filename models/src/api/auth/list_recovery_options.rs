use crate::prelude::*;

macros::declare_api_endpoint!(
	// List all recovery options
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
