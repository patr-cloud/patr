use serde::{Deserialize, Serialize};

mod api_token;
mod change_password;
mod get_user_details; // Get a user's details by userId
mod get_user_info; // Get my own personal info
mod list_workspaces;
mod logins;
mod recovery_options;
mod search_for_user;
mod set_user_info; // Set my own personal info

pub use self::{
	api_token::*,
	change_password::*,
	get_user_details::*,
	get_user_info::*,
	list_workspaces::*,
	logins::*,
	recovery_options::*,
	search_for_user::*,
	set_user_info::*,
};
use crate::utils::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserPhoneNumber {
	pub country_code: String,
	pub phone_number: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BasicUserInfo {
	pub id: Uuid,
	pub username: String,
	pub first_name: String,
	pub last_name: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub bio: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub location: Option<String>,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{BasicUserInfo, UserPhoneNumber};
	use crate::utils::Uuid;

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
				id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
					.unwrap(),
				username: "john-patr".to_string(),
				first_name: "John".to_string(),
				last_name: "Patr".to_string(),
				bio: None,
				location: None,
			},
			&[
				Token::Struct {
					name: "BasicUserInfo",
					len: 4,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
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

	#[test]
	fn assert_basic_user_info_types_with_bio() {
		assert_tokens(
			&BasicUserInfo {
				id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
					.unwrap(),
				username: "john-patr".to_string(),
				first_name: "John".to_string(),
				last_name: "Patr".to_string(),
				bio: Some("I'm a random bot".to_string()),
				location: None,
			},
			&[
				Token::Struct {
					name: "BasicUserInfo",
					len: 5,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("username"),
				Token::Str("john-patr"),
				Token::Str("firstName"),
				Token::Str("John"),
				Token::Str("lastName"),
				Token::Str("Patr"),
				Token::Str("bio"),
				Token::Some,
				Token::Str("I'm a random bot"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_basic_user_info_types_with_location() {
		assert_tokens(
			&BasicUserInfo {
				id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
					.unwrap(),
				username: "john-patr".to_string(),
				first_name: "John".to_string(),
				last_name: "Patr".to_string(),
				bio: None,
				location: Some("Somewhere in the internet".to_string()),
			},
			&[
				Token::Struct {
					name: "BasicUserInfo",
					len: 5,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("username"),
				Token::Str("john-patr"),
				Token::Str("firstName"),
				Token::Str("John"),
				Token::Str("lastName"),
				Token::Str("Patr"),
				Token::Str("location"),
				Token::Some,
				Token::Str("Somewhere in the internet"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_basic_user_info_types_with_bio_and_location() {
		assert_tokens(
			&BasicUserInfo {
				id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
					.unwrap(),
				username: "john-patr".to_string(),
				first_name: "John".to_string(),
				last_name: "Patr".to_string(),
				bio: Some("I'm a random bot".to_string()),
				location: Some("Somewhere in the internet".to_string()),
			},
			&[
				Token::Struct {
					name: "BasicUserInfo",
					len: 6,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("username"),
				Token::Str("john-patr"),
				Token::Str("firstName"),
				Token::Str("John"),
				Token::Str("lastName"),
				Token::Str("Patr"),
				Token::Str("bio"),
				Token::Some,
				Token::Str("I'm a random bot"),
				Token::Str("location"),
				Token::Some,
				Token::Str("Somewhere in the internet"),
				Token::StructEnd,
			],
		);
	}
}
