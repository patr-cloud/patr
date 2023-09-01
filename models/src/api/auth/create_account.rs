use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum RecoveryMethod {
	#[serde(rename_all = "camelCase")]
	PhoneNumber {
		recovery_phone_country_code: String,
		recovery_phone_number: String,
	},
	#[serde(rename_all = "camelCase")]
	Email { recovery_email: String },
}

macros::declare_api_endpoint!(
	CreateAccount,
	POST "/auth/sign-up",
	request = {
		pub username: String,
		pub password: String,
		pub first_name: String,
		pub last_name: String,
		#[serde(flatten)]
		pub recovery_method: RecoveryMethod,
	},
);
