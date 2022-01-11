use api_models::utils::Uuid;
use serde::{Deserialize, Serialize};

use crate::utils::constants::ResourceOwnerType;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct User {
	pub id: Uuid,
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
	pub backup_email_domain_id: Option<Uuid>,

	#[serde(skip)]
	pub backup_phone_country_code: Option<String>,
	#[serde(skip)]
	pub backup_phone_number: Option<String>,
}

pub struct UserLogin {
	pub login_id: Uuid,
	/// Hashed refresh token
	pub refresh_token: String,
	pub token_expiry: u64,
	pub user_id: Uuid,
	pub last_login: u64,
	pub last_activity: u64,
}

#[derive(Clone)]
pub struct UserEmailAddress {
	pub email_local: String,
	pub domain_id: Uuid,
}

pub struct UserToSignUp {
	pub username: String,
	pub account_type: ResourceOwnerType,

	pub password: String,
	pub first_name: String,
	pub last_name: String,

	pub backup_email_local: Option<String>,
	pub backup_email_domain_id: Option<Uuid>,

	pub backup_phone_country_code: Option<String>,
	pub backup_phone_number: Option<String>,

	pub business_email_local: Option<String>,
	pub business_domain_name: Option<String>,
	pub business_name: Option<String>,

	pub otp_hash: String,
	pub otp_expiry: u64,
}

pub struct PasswordResetRequest {
	pub user_id: Uuid,
	pub token: String,
	pub token_expiry: u64,
}

pub struct PersonalEmailToBeVerified {
	pub local: String,
	pub domain_id: Uuid,
	pub user_id: Uuid,
	pub verification_token_hash: String,
	pub verification_token_expiry: u64,
}

pub struct PhoneNumberToBeVerified {
	pub country_code: String,
	pub phone_number: String,
	pub user_id: Uuid,
	pub verification_token_hash: String,
	pub verification_token_expiry: u64,
}

pub struct PhoneCountryCode {
	pub country_code: String,
	pub phone_code: String,
	pub country_name: String,
}

pub struct JoinUser {
	pub jwt: String,
	pub login_id: Uuid,
	pub refresh_token: Uuid,
	pub welcome_email_to: Option<String>,
	pub backup_email_to: Option<String>,
	pub backup_phone_number_to: Option<String>,
}
