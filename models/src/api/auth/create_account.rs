use crate::prelude::*;
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

macros::declare_api_endpoint!(
	// Create a new account
	CreateAccount,
	POST "/auth/sign-up",
	request = {
		pub username: String,
		pub password: String,
		pub first_name: String,
		pub last_name: String,
		#[serde(flatten)]
		pub recovery_method: RecoveryMethod,
		#[serde(flatten)]
		pub account_type: SignUpAccountType,
		#[serde(skip_serializing_if = "Option::is_none")]
		pub coupon_code: Option<String>,
	},
);
