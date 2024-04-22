use serde::{Deserialize, Serialize};

use crate::prelude::*;

/// Recovery method options provided to the user when they forget their
/// passsword and request a password change by hitting the ForgetPassword API
/// endpoint. The curent recovery options are email and phone number.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
#[preprocess::sync]
pub enum RecoveryMethod {
	#[serde(rename_all = "camelCase")]
	/// Phone number
	PhoneNumber {
		/// The country code of the phone number. Example: US, IN, etc.
		/// POLICY:
		/// Plus sign followed by 1 to 4 digits
		#[preprocess(trim, regex = r"^\+\d{1,4}$")]
		recovery_phone_country_code: String,
		/// The phone number of the user
		/// POLICY:
		/// Standard 10-digit format
		#[preprocess(trim, regex = r"^\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}$")]
		recovery_phone_number: String,
	},
	#[serde(rename_all = "camelCase")]
	/// Email
	Email {
		/// The email address of the user
		#[preprocess(email)]
		recovery_email: String,
	},
}

macros::declare_api_endpoint!(
	/// The route to create a new user account
	CreateAccount,
	POST "/auth/sign-up",
	api = false,
	request_headers = {
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	request = {
		/// The username of the user signing up
		#[preprocess(length(min = 4, max = 10), trim, lowercase, regex = "^[a-z0-9_][a-z0-9_\\.\\-]*[a-z0-9_]$")]
		pub username: String,
		/// The password
		/// POLICY:
		/// Minimum length (often at least 8 characters).
		/// At least one uppercase letter.
		/// At least one lowercase letter.
		/// At least one digit.
		/// At least one special character (e.g., !@#$%^&*)
		#[preprocess(trim, regex = r"^(?:.*[a-z])(?:.*[A-Z])(?:.*\d)(?:.*[@$!%*?&])[A-Za-z\d@$!%*?&]{8,}$")]
		pub password: String,
		/// The first name of the user
		#[preprocess(trim)]
		pub first_name: String,
		/// The last name of the user
		#[preprocess(trim)]
		pub last_name: String,
		/// The recovery method the user would recover their account with
		#[serde(flatten)]
		pub recovery_method: RecoveryMethod,
	},
);
