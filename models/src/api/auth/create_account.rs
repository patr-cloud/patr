use crate::prelude::*;
use serde::{Deserialize, Serialize};

/// Recovery method options provided to the users
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum RecoveryMethod {
	#[serde(rename_all = "camelCase")]
	/// Phone number
	PhoneNumber {
		/// The country code of the phone number. Example: US, IN, etc.
		recovery_phone_country_code: String,
		/// The phone number of the user
		recovery_phone_number: String,
	},
	#[serde(rename_all = "camelCase")]
	/// Email
	Email {
		/// The email address of the user
		recovery_email: String,
	},
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
		/// The username of the user signing up
		pub username: String,
		/// The password
		pub password: String,
		/// The first name of the user
		pub first_name: String,
		/// The last name of the user
		pub last_name: String,
		/// The recovery method the user would recover their account with
		#[serde(flatten)]
		pub recovery_method: RecoveryMethod,
		#[serde(flatten)]
		pub account_type: SignUpAccountType,
		#[serde(skip_serializing_if = "Option::is_none")]
		pub coupon_code: Option<String>,
	},
);
