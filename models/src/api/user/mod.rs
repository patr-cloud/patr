use serde::{Deserialize, Serialize};

mod api_token;
mod change_password;
mod get_user_details; // Get a user's details by userId
mod get_user_info; // Get my own personal info
mod list_workspaces;
mod recovery_options;
mod set_user_info;
mod web_logins; // Set my own personal info

pub use self::{
	api_token::*,
	change_password::*,
	get_user_details::*,
	get_user_info::*,
	list_workspaces::*,
	recovery_options::*,
	set_user_info::*,
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

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{BasicUserInfo, UserPhoneNumber};

	#[test]
	fn assert_user_phone_number_types() {
		assert_tokens(
			&UserPhoneNumber {
				country_code: "IN".to_string(),
				phone_number: "1234567890".to_string(),
			},
			&[
				Token::Struct {
					name: "UserPhoneNumber",
					len: 2,
				},
				Token::Str("countryCode"),
				Token::Str("IN"),
				Token::Str("phoneNumber"),
				Token::Str("1234567890"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_basic_user_info_types() {
		assert_tokens(
			&BasicUserInfo {
				username: "john-patr".to_string(),
				first_name: "John".to_string(),
				last_name: "Patr".to_string(),
			},
			&[
				Token::Struct {
					name: "BasicUserInfo",
					len: 3,
				},
				Token::Str("username"),
				Token::Str("john-patr"),
				Token::Str("firstName"),
				Token::Str("John"),
				Token::Str("lastName"),
				Token::Str("Patr"),
				Token::StructEnd,
			],
		);
	}
}
