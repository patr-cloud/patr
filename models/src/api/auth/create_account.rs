use serde::{Deserialize, Serialize};

/// Recovery method options provided to the user when they forget their
/// passsword and request a password change by hitting the ForgetPassword API
/// endpoint. The curent recovery options are email and phone number.
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

macros::declare_api_endpoint!(
	/// The route to create a new user account
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
	},
);
