use eve_rs::AsError;
use sqlx::Transaction;

use crate::{
	db, error,
	models::{
		db_mapping::{User, UserLogin},
		rbac, AccessTokenData, ExposedUserData,
	},
	service::{self, get_refresh_token_expiry},
	utils::{
		constants::ResourceOwnerType, get_current_time_millis,
		settings::Settings, validator, Error,
	},
	Database,
};

mod email;
mod sms;

pub use email::*;
pub use sms::*;

// could possibly also take in `PreferredNotifierType`
pub async fn send_user_verification_otp(
	country_code: Option<&str>,
	phone_number: Option<&str>,
	email: Option<&str>,
	otp: &str,
) -> Result<(), Error> {
	log::error!("NOTIFIER NOT YET IMPLEMENTED, Thanks for trying LOL");
	// TODO: check if any None value is passed before unwrapping directly
	if phone_number.is_some() {
		sms::send_user_verification_otp(
			country_code.unwrap(),
			phone_number.unwrap(),
			otp,
		)?;
	}

	if email.is_some() {
		email::send_user_verification_otp(email.unwrap(), otp)?;
	}
	Ok(())
}

// reset password
pub async fn send_user_reset_password_notification(
	country_code: Option<&str>,
	phone_number: Option<&str>,
	email: Option<&str>,
) -> Result<(), Error> {
	log::error!("NOTIFIER NOT YET IMPLEMENTED, Thanks for trying LOL");
	if phone_number.is_some() {
		sms::send_user_reset_password_notification(
			country_code.unwrap(),
			phone_number.unwrap(),
		)?;
	}

	if email.is_some() {
		email::send_user_reset_password_notification(email.unwrap())?;
	}
	Ok(())
}
