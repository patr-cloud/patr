use serde::{Deserialize, Serialize};

use crate::utils::{Business, Personal, ResourceType};

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum SignUpAccountType {
	#[serde(rename_all = "camelCase")]
	Personal { account_type: Personal },
	#[serde(rename_all = "camelCase")]
	Business {
		account_type: Business,
		workspace_name: String,
		business_email_local: String,
		domain: String,
	},
}

impl SignUpAccountType {
	pub fn is_personal(&self) -> bool {
		matches!(self, Self::Personal { .. })
	}

	pub fn is_business(&self) -> bool {
		matches!(self, Self::Business { .. })
	}
}

impl SignUpAccountType {
	pub fn account_type(&self) -> ResourceType {
		match self {
			Self::Personal { .. } => ResourceType::Personal,
			Self::Business { .. } => ResourceType::Business,
		}
	}
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
