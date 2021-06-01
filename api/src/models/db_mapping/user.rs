use std::str::FromStr;

use eve_rs::AsError;
use serde::{Deserialize, Serialize};

use crate::{
	error,
	utils::{constants::ResourceOwnerType, Error},
};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct User {
	pub id: Vec<u8>,
	pub username: String,
	#[serde(skip)]
	pub password: String,
	pub first_name: String,
	pub last_name: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub dob: Option<u64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub bio: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub location: Option<String>,
	pub created: u64,

	#[serde(skip)]
	pub backup_email_local: Option<String>,
	#[serde(skip)]
	pub backup_email_domain_id: Option<Vec<u8>>,

	#[serde(skip)]
	pub backup_phone_country_code: Option<String>,
	#[serde(skip)]
	pub backup_phone_number: Option<String>,
}

pub struct UserLogin {
	pub login_id: Vec<u8>,
	pub refresh_token: String,
	pub token_expiry: u64,
	pub user_id: Vec<u8>,
	pub last_login: u64,
	pub last_activity: u64,
}

#[derive(Clone)]
pub struct UserEmailAddress {
	pub email_local: String,
	pub domain_id: Vec<u8>,
}

#[allow(dead_code)]
pub struct UserPhoneNumber {
	pub user_id: Vec<u8>,
	pub country_code: String,
	pub number: String,
}

pub struct UserToSignUp {
	pub username: String,
	pub account_type: ResourceOwnerType,

	pub password: String,
	pub first_name: String,
	pub last_name: String,

	pub backup_email_local: Option<String>,
	pub backup_email_domain_id: Option<Vec<u8>>,

	pub backup_phone_country_code: Option<String>,
	pub backup_phone_number: Option<String>,

	pub org_email_local: Option<String>,
	pub org_domain_name: Option<String>,
	pub organisation_name: Option<String>,

	pub otp_hash: String,
	pub otp_expiry: u64,
}

pub struct PasswordResetRequest {
	pub user_id: Vec<u8>,
	pub token: String,
	pub token_expiry: u64,
}

pub struct PersonalEmailToBeVerified {
	pub local: String,
	pub domain_id: Vec<u8>,
	pub user_id: Vec<u8>,
	pub verification_token_hash: String,
	pub verification_token_expiry: u64,
}

pub struct PhoneCountryCode {
	pub country_code: String,
	pub phone_code: String,
	pub country_name: String,
}

pub enum JoinNotifier {
	WelcomeEmail,
	BackupEmail,
	BackupPhoneNumber,
}
// enum taken in as response from the front end
#[derive(sqlx::Type, Debug)]
#[sqlx(type_name = "RESOURCE_OWNER_TYPE", rename_all = "lowercase")]
pub enum PreferredRecoveryOption {
	BackupPhoneNumber,
	BackupEmail,
}

impl FromStr for PreferredRecoveryOption {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.to_lowercase().as_str() {
			"phonenumber" => Ok(PreferredRecoveryOption::BackupPhoneNumber),
			"email" => Ok(PreferredRecoveryOption::BackupEmail),
			_ => Error::as_result()
				.status(500)
				.body(error!(WRONG_PARAMETERS).to_string()),
		}
	}
}
