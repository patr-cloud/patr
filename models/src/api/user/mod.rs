use serde::{Deserialize, Serialize};

mod api_token;
mod change_password;
mod get_user_details; // Get a user's details by userId
mod get_user_info; // Get my own personal info
mod list_user_workspaces;
mod recovery_options;
mod update_user_info;
mod web_logins; // Set my own personal info

mod mfa;

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

/// This is the information that is _allowed_ to be public about a user. This is
/// not the entire user object, but only the information that is allowed to be
/// public. For privacy reasons, things like their email address and phone
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
