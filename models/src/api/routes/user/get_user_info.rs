use axum_extra::routing::TypedPath;
use chrono::Utc;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::{BasicUserInfo, UserPhoneNumber};
use crate::{utils::DateTime, ApiRequest};

#[derive(
	Eq,
	Ord,
	Copy,
	Hash,
	Debug,
	Clone,
	Default,
	TypedPath,
	PartialEq,
	Serialize,
	PartialOrd,
	Deserialize,
)]
#[typed_path("/user/info")]
pub struct GetUserInfoPath;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetUserInfoRequest;

impl ApiRequest for GetUserInfoRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = GetUserInfoPath;
	type RequestQuery = ();
	type RequestBody = ();
	type Response = GetUserInfoResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetUserInfoResponse {
	#[serde(flatten)]
	pub basic_user_info: BasicUserInfo,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub birthday: Option<DateTime<Utc>>,
	pub is_password_set: bool,
	pub created: DateTime<Utc>,
	pub recovery_email: Option<String>,
	pub secondary_emails: Vec<String>,
	pub recovery_phone_number: Option<UserPhoneNumber>,
	pub secondary_phone_numbers: Vec<UserPhoneNumber>,
}

#[cfg(test)]
mod test {
	use chrono::{TimeZone, Utc};
	use serde_test::{assert_tokens, Token};

	use super::{GetUserInfoRequest, GetUserInfoResponse};
	use crate::{
		models::user::{BasicUserInfo, UserPhoneNumber},
		utils::Uuid,
		ApiResponse,
	};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&GetUserInfoRequest,
			&[Token::UnitStruct {
				name: "GetUserInfoRequest",
			}],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&GetUserInfoResponse {
				basic_user_info: BasicUserInfo {
					id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					username: "john-patr".to_string(),
					first_name: "John".to_string(),
					last_name: "Patr".to_string(),
					bio: None,
					location: None,
				},
				birthday: None,
				is_password_set: true,
				created: Utc.timestamp_opt(0, 0).unwrap().into(),
				recovery_email: Some("johnpatr@gmail.com".to_string()),
				secondary_emails: vec![],
				recovery_phone_number: Some(UserPhoneNumber {
					country_code: "IN".to_string(),
					phone_number: "1234567890".to_string(),
				}),
				secondary_phone_numbers: vec![],
			},
			&[
				Token::Map { len: None },
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("username"),
				Token::Str("john-patr"),
				Token::Str("firstName"),
				Token::Str("John"),
				Token::Str("lastName"),
				Token::Str("Patr"),
				Token::Str("isPasswordSet"),
				Token::Bool(true),
				Token::Str("created"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("recoveryEmail"),
				Token::Some,
				Token::Str("johnpatr@gmail.com"),
				Token::Str("secondaryEmails"),
				Token::Seq { len: Some(0) },
				Token::SeqEnd,
				Token::Str("recoveryPhoneNumber"),
				Token::Some,
				Token::Struct {
					name: "UserPhoneNumber",
					len: 2,
				},
				Token::Str("countryCode"),
				Token::Str("IN"),
				Token::Str("phoneNumber"),
				Token::Str("1234567890"),
				Token::StructEnd,
				Token::Str("secondaryPhoneNumbers"),
				Token::Seq { len: Some(0) },
				Token::SeqEnd,
				Token::MapEnd,
			],
		);
	}

	#[test]
	fn assert_response_types_with_bio() {
		assert_tokens(
			&GetUserInfoResponse {
				basic_user_info: BasicUserInfo {
					id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					username: "john-patr".to_string(),
					first_name: "John".to_string(),
					last_name: "Patr".to_string(),
					bio: Some("I'm a random bot".to_string()),
					location: None,
				},
				birthday: None,
				is_password_set: true,
				created: Utc.timestamp_opt(0, 0).unwrap().into(),
				recovery_email: Some("johnpatr@gmail.com".to_string()),
				secondary_emails: vec![],
				recovery_phone_number: Some(UserPhoneNumber {
					country_code: "IN".to_string(),
					phone_number: "1234567890".to_string(),
				}),
				secondary_phone_numbers: vec![],
			},
			&[
				Token::Map { len: None },
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
				Token::Str("isPasswordSet"),
				Token::Bool(true),
				Token::Str("created"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("recoveryEmail"),
				Token::Some,
				Token::Str("johnpatr@gmail.com"),
				Token::Str("secondaryEmails"),
				Token::Seq { len: Some(0) },
				Token::SeqEnd,
				Token::Str("recoveryPhoneNumber"),
				Token::Some,
				Token::Struct {
					name: "UserPhoneNumber",
					len: 2,
				},
				Token::Str("countryCode"),
				Token::Str("IN"),
				Token::Str("phoneNumber"),
				Token::Str("1234567890"),
				Token::StructEnd,
				Token::Str("secondaryPhoneNumbers"),
				Token::Seq { len: Some(0) },
				Token::SeqEnd,
				Token::MapEnd,
			],
		);
	}

	#[test]
	fn assert_response_types_with_location() {
		assert_tokens(
			&GetUserInfoResponse {
				basic_user_info: BasicUserInfo {
					id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					username: "john-patr".to_string(),
					first_name: "John".to_string(),
					last_name: "Patr".to_string(),
					bio: None,
					location: Some("Somewhere in the internet".to_string()),
				},
				birthday: None,
				is_password_set: true,
				created: Utc.timestamp_opt(0, 0).unwrap().into(),
				recovery_email: Some("johnpatr@gmail.com".to_string()),
				secondary_emails: vec![],
				recovery_phone_number: Some(UserPhoneNumber {
					country_code: "IN".to_string(),
					phone_number: "1234567890".to_string(),
				}),
				secondary_phone_numbers: vec![],
			},
			&[
				Token::Map { len: None },
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
				Token::Str("isPasswordSet"),
				Token::Bool(true),
				Token::Str("created"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("recoveryEmail"),
				Token::Some,
				Token::Str("johnpatr@gmail.com"),
				Token::Str("secondaryEmails"),
				Token::Seq { len: Some(0) },
				Token::SeqEnd,
				Token::Str("recoveryPhoneNumber"),
				Token::Some,
				Token::Struct {
					name: "UserPhoneNumber",
					len: 2,
				},
				Token::Str("countryCode"),
				Token::Str("IN"),
				Token::Str("phoneNumber"),
				Token::Str("1234567890"),
				Token::StructEnd,
				Token::Str("secondaryPhoneNumbers"),
				Token::Seq { len: Some(0) },
				Token::SeqEnd,
				Token::MapEnd,
			],
		);
	}

	#[test]
	fn assert_response_types_with_birthday() {
		assert_tokens(
			&GetUserInfoResponse {
				basic_user_info: BasicUserInfo {
					id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					username: "john-patr".to_string(),
					first_name: "John".to_string(),
					last_name: "Patr".to_string(),
					bio: None,
					location: None,
				},
				birthday: Some(Utc.timestamp_opt(0, 0).unwrap().into()),
				is_password_set: true,
				created: Utc.timestamp_opt(0, 0).unwrap().into(),
				recovery_email: Some("johnpatr@gmail.com".to_string()),
				secondary_emails: vec![],
				recovery_phone_number: Some(UserPhoneNumber {
					country_code: "IN".to_string(),
					phone_number: "1234567890".to_string(),
				}),
				secondary_phone_numbers: vec![],
			},
			&[
				Token::Map { len: None },
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("username"),
				Token::Str("john-patr"),
				Token::Str("firstName"),
				Token::Str("John"),
				Token::Str("lastName"),
				Token::Str("Patr"),
				Token::Str("birthday"),
				Token::Some,
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("isPasswordSet"),
				Token::Bool(true),
				Token::Str("created"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("recoveryEmail"),
				Token::Some,
				Token::Str("johnpatr@gmail.com"),
				Token::Str("secondaryEmails"),
				Token::Seq { len: Some(0) },
				Token::SeqEnd,
				Token::Str("recoveryPhoneNumber"),
				Token::Some,
				Token::Struct {
					name: "UserPhoneNumber",
					len: 2,
				},
				Token::Str("countryCode"),
				Token::Str("IN"),
				Token::Str("phoneNumber"),
				Token::Str("1234567890"),
				Token::StructEnd,
				Token::Str("secondaryPhoneNumbers"),
				Token::Seq { len: Some(0) },
				Token::SeqEnd,
				Token::MapEnd,
			],
		);
	}

	#[test]
	fn assert_response_types_with_bio_and_location_and_birthday() {
		assert_tokens(
			&GetUserInfoResponse {
				basic_user_info: BasicUserInfo {
					id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					username: "john-patr".to_string(),
					first_name: "John".to_string(),
					last_name: "Patr".to_string(),
					bio: Some("I'm a random bot".to_string()),
					location: Some("Somewhere in the internet".to_string()),
				},
				birthday: Some(Utc.timestamp_opt(0, 0).unwrap().into()),
				is_password_set: true,
				created: Utc.timestamp_opt(0, 0).unwrap().into(),
				recovery_email: Some("johnpatr@gmail.com".to_string()),
				secondary_emails: vec![],
				recovery_phone_number: Some(UserPhoneNumber {
					country_code: "IN".to_string(),
					phone_number: "1234567890".to_string(),
				}),
				secondary_phone_numbers: vec![],
			},
			&[
				Token::Map { len: None },
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
				Token::Str("birthday"),
				Token::Some,
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("isPasswordSet"),
				Token::Bool(true),
				Token::Str("created"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("recoveryEmail"),
				Token::Some,
				Token::Str("johnpatr@gmail.com"),
				Token::Str("secondaryEmails"),
				Token::Seq { len: Some(0) },
				Token::SeqEnd,
				Token::Str("recoveryPhoneNumber"),
				Token::Some,
				Token::Struct {
					name: "UserPhoneNumber",
					len: 2,
				},
				Token::Str("countryCode"),
				Token::Str("IN"),
				Token::Str("phoneNumber"),
				Token::Str("1234567890"),
				Token::StructEnd,
				Token::Str("secondaryPhoneNumbers"),
				Token::Seq { len: Some(0) },
				Token::SeqEnd,
				Token::MapEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(GetUserInfoResponse {
				basic_user_info: BasicUserInfo {
					id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					username: "john-patr".to_string(),
					first_name: "John".to_string(),
					last_name: "Patr".to_string(),
					bio: None,
					location: None,
				},
				birthday: None,
				is_password_set: true,
				created: Utc.timestamp_opt(0, 0).unwrap().into(),
				recovery_email: Some("johnpatr@gmail.com".to_string()),
				secondary_emails: vec![],
				recovery_phone_number: Some(UserPhoneNumber {
					country_code: "IN".to_string(),
					phone_number: "1234567890".to_string(),
				}),
				secondary_phone_numbers: vec![],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("username"),
				Token::Str("john-patr"),
				Token::Str("firstName"),
				Token::Str("John"),
				Token::Str("lastName"),
				Token::Str("Patr"),
				Token::Str("isPasswordSet"),
				Token::Bool(true),
				Token::Str("created"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("recoveryEmail"),
				Token::Some,
				Token::Str("johnpatr@gmail.com"),
				Token::Str("secondaryEmails"),
				Token::Seq { len: Some(0) },
				Token::SeqEnd,
				Token::Str("recoveryPhoneNumber"),
				Token::Some,
				Token::Struct {
					name: "UserPhoneNumber",
					len: 2,
				},
				Token::Str("countryCode"),
				Token::Str("IN"),
				Token::Str("phoneNumber"),
				Token::Str("1234567890"),
				Token::StructEnd,
				Token::Str("secondaryPhoneNumbers"),
				Token::Seq { len: Some(0) },
				Token::SeqEnd,
				Token::MapEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types_with_bio() {
		assert_tokens(
			&ApiResponse::success(GetUserInfoResponse {
				basic_user_info: BasicUserInfo {
					id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					username: "john-patr".to_string(),
					first_name: "John".to_string(),
					last_name: "Patr".to_string(),
					bio: Some("I'm a random bot".to_string()),
					location: None,
				},
				birthday: None,
				is_password_set: true,
				created: Utc.timestamp_opt(0, 0).unwrap().into(),
				recovery_email: Some("johnpatr@gmail.com".to_string()),
				secondary_emails: vec![],
				recovery_phone_number: Some(UserPhoneNumber {
					country_code: "IN".to_string(),
					phone_number: "1234567890".to_string(),
				}),
				secondary_phone_numbers: vec![],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
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
				Token::Str("isPasswordSet"),
				Token::Bool(true),
				Token::Str("created"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("recoveryEmail"),
				Token::Some,
				Token::Str("johnpatr@gmail.com"),
				Token::Str("secondaryEmails"),
				Token::Seq { len: Some(0) },
				Token::SeqEnd,
				Token::Str("recoveryPhoneNumber"),
				Token::Some,
				Token::Struct {
					name: "UserPhoneNumber",
					len: 2,
				},
				Token::Str("countryCode"),
				Token::Str("IN"),
				Token::Str("phoneNumber"),
				Token::Str("1234567890"),
				Token::StructEnd,
				Token::Str("secondaryPhoneNumbers"),
				Token::Seq { len: Some(0) },
				Token::SeqEnd,
				Token::MapEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types_with_location() {
		assert_tokens(
			&ApiResponse::success(GetUserInfoResponse {
				basic_user_info: BasicUserInfo {
					id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					username: "john-patr".to_string(),
					first_name: "John".to_string(),
					last_name: "Patr".to_string(),
					bio: None,
					location: Some("Somewhere in the internet".to_string()),
				},
				birthday: None,
				is_password_set: true,
				created: Utc.timestamp_opt(0, 0).unwrap().into(),
				recovery_email: Some("johnpatr@gmail.com".to_string()),
				secondary_emails: vec![],
				recovery_phone_number: Some(UserPhoneNumber {
					country_code: "IN".to_string(),
					phone_number: "1234567890".to_string(),
				}),
				secondary_phone_numbers: vec![],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
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
				Token::Str("isPasswordSet"),
				Token::Bool(true),
				Token::Str("created"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("recoveryEmail"),
				Token::Some,
				Token::Str("johnpatr@gmail.com"),
				Token::Str("secondaryEmails"),
				Token::Seq { len: Some(0) },
				Token::SeqEnd,
				Token::Str("recoveryPhoneNumber"),
				Token::Some,
				Token::Struct {
					name: "UserPhoneNumber",
					len: 2,
				},
				Token::Str("countryCode"),
				Token::Str("IN"),
				Token::Str("phoneNumber"),
				Token::Str("1234567890"),
				Token::StructEnd,
				Token::Str("secondaryPhoneNumbers"),
				Token::Seq { len: Some(0) },
				Token::SeqEnd,
				Token::MapEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types_with_birthday() {
		assert_tokens(
			&ApiResponse::success(GetUserInfoResponse {
				basic_user_info: BasicUserInfo {
					id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					username: "john-patr".to_string(),
					first_name: "John".to_string(),
					last_name: "Patr".to_string(),
					bio: None,
					location: None,
				},
				birthday: Some(Utc.timestamp_opt(0, 0).unwrap().into()),
				is_password_set: true,
				created: Utc.timestamp_opt(0, 0).unwrap().into(),
				recovery_email: Some("johnpatr@gmail.com".to_string()),
				secondary_emails: vec![],
				recovery_phone_number: Some(UserPhoneNumber {
					country_code: "IN".to_string(),
					phone_number: "1234567890".to_string(),
				}),
				secondary_phone_numbers: vec![],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("username"),
				Token::Str("john-patr"),
				Token::Str("firstName"),
				Token::Str("John"),
				Token::Str("lastName"),
				Token::Str("Patr"),
				Token::Str("birthday"),
				Token::Some,
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("isPasswordSet"),
				Token::Bool(true),
				Token::Str("created"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("recoveryEmail"),
				Token::Some,
				Token::Str("johnpatr@gmail.com"),
				Token::Str("secondaryEmails"),
				Token::Seq { len: Some(0) },
				Token::SeqEnd,
				Token::Str("recoveryPhoneNumber"),
				Token::Some,
				Token::Struct {
					name: "UserPhoneNumber",
					len: 2,
				},
				Token::Str("countryCode"),
				Token::Str("IN"),
				Token::Str("phoneNumber"),
				Token::Str("1234567890"),
				Token::StructEnd,
				Token::Str("secondaryPhoneNumbers"),
				Token::Seq { len: Some(0) },
				Token::SeqEnd,
				Token::MapEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types_with_bio_and_location_and_birthday() {
		assert_tokens(
			&ApiResponse::success(GetUserInfoResponse {
				basic_user_info: BasicUserInfo {
					id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					username: "john-patr".to_string(),
					first_name: "John".to_string(),
					last_name: "Patr".to_string(),
					bio: Some("I'm a random bot".to_string()),
					location: Some("Somewhere in the internet".to_string()),
				},
				birthday: Some(Utc.timestamp_opt(0, 0).unwrap().into()),
				is_password_set: true,
				created: Utc.timestamp_opt(0, 0).unwrap().into(),
				recovery_email: Some("johnpatr@gmail.com".to_string()),
				secondary_emails: vec![],
				recovery_phone_number: Some(UserPhoneNumber {
					country_code: "IN".to_string(),
					phone_number: "1234567890".to_string(),
				}),
				secondary_phone_numbers: vec![],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
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
				Token::Str("birthday"),
				Token::Some,
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("isPasswordSet"),
				Token::Bool(true),
				Token::Str("created"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("recoveryEmail"),
				Token::Some,
				Token::Str("johnpatr@gmail.com"),
				Token::Str("secondaryEmails"),
				Token::Seq { len: Some(0) },
				Token::SeqEnd,
				Token::Str("recoveryPhoneNumber"),
				Token::Some,
				Token::Struct {
					name: "UserPhoneNumber",
					len: 2,
				},
				Token::Str("countryCode"),
				Token::Str("IN"),
				Token::Str("phoneNumber"),
				Token::Str("1234567890"),
				Token::StructEnd,
				Token::Str("secondaryPhoneNumbers"),
				Token::Seq { len: Some(0) },
				Token::SeqEnd,
				Token::MapEnd,
			],
		);
	}
}
