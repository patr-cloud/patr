use serde::{Deserialize, Serialize};

/// All endpoints related to API tokens
mod api_token;
/// The endpoint to change the password of a user
mod change_password;
/// The endpoint to get the details of any user, based on their userId
mod get_user_details;
/// The endpoint to get the details of the currently logged in user
mod get_user_info;
/// The endpoint to list all the workspaces that a user is a part of
mod list_user_workspaces;
/// All endpoints related to MFA
mod mfa;
/// All endpoints related to recovery options
mod recovery_options;
/// The endpoint to update the information of a user
mod update_user_info;
/// All endpoints related to web logins
mod web_logins;

pub use self::{
	api_token::*,
	change_password::*,
	get_user_details::*,
	get_user_info::*,
	list_user_workspaces::*,
	mfa::*,
	recovery_options::*,
	update_user_info::*,
	web_logins::*,
};

/// The phone number of a user. This is used to send OTPs, notifications, etc to
/// the user.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserPhoneNumber {
	/// The country code of the phone number. This is a 2 letter code, such as
	/// IN, US, UK, etc.
	pub country_code: String,
	/// The phone number of the user. This should be a valid phone number. This
	/// is a string because it can contain leading zeroes, which will not be
	/// preserved if it is an integer.
	pub phone_number: String,
}

/// This is the information that is _allowed_ to be public about a user.
///
/// This is not the entire user object, but only the information that is allowed
/// to be public. For privacy reasons, things like their email address and phone
/// number are not public.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BasicUserInfo {
	/// The username of the user. This is unique to the user.
	pub username: String,
	/// The first name of the user.
	pub first_name: String,
	/// The last name of the user.
	pub last_name: String,
}
