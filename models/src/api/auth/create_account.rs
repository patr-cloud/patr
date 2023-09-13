use crate::prelude::*;
use serde::{Deserialize, Serialize};

// Recovery method options provided to the users
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum RecoveryMethod {
	#[serde(rename_all = "camelCase")]
	// Phone number
	PhoneNumber {
		recovery_phone_country_code: String,
		recovery_phone_number: String,
	},
	#[serde(rename_all = "camelCase")]
	// Email
	Email { recovery_email: String },
}

macros::declare_api_endpoint!(
	// Definition of a route to create a new user account
	CreateAccount,
	POST "/auth/sign-up",
	request = {
		// The username to be created
		pub username: String,
		// The password
		pub password: String,
		// The first name of the user
		pub first_name: String,
		// The last name of the user
		pub last_name: String,
		// The recovery method the user would recover their account with
		#[serde(flatten)]
		pub recovery_method: RecoveryMethod,
		// The coupon code the user signs-up with
		#[serde(skip_serializing_if = "Option::is_none")]
		pub coupon_code: Option<String>,
	},
);
