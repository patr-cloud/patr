use eve_rs::AsError;
use sqlx::Transaction;

use crate::{
	db,
	error,
	models::{
		db_mapping::{PreferredRecoveryOption, User, UserLogin},
		rbac,
		AccessTokenData,
		ExposedUserData,
	},
	service::{self, get_refresh_token_expiry},
	utils::{
		constants::ResourceOwnerType,
		get_current_time_millis,
		settings::Settings,
		validator,
		Error,
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
		sms::send_user_verification_otp(phone_number.unwrap(), otp)?;
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

pub async fn send_forgot_password_otp(
	connection: &mut Transaction<'_, Database>,
	user_id: &str,
	recovery_option: PreferredRecoveryOption,
	otp: &str,
) -> Result<(), Error> {
	let user = db::get_user_by_username_or_email(connection, user_id).await?;

	if user.is_none() {
		Error::as_result()
			.status(200)
			.body(error!(USER_NOT_FOUND).to_string())?;
	}
	let user = user.unwrap();

	// match on the recovery type
	match recovery_option {
		PreferredRecoveryOption::BackupEmail => {
			let domain = db::get_personal_domain_by_id(
				connection,
				user.backup_email_domain_id.unwrap().as_ref(),
			)
			.await?
			.status(500)?;

			let email =
				format!("{}@{}", user.backup_email_local.unwrap(), domain.name);
			// send email
			email::send_user_verification_otp(&email, otp)?;
			// panic!("SENDING OTP THROUGH EMAIL FOR FORGOT PASSWORD IS NOT
			// IMPLEMENTED YET.");
		}
		PreferredRecoveryOption::BackupPhoneNumber => {
			let phone_number = user.backup_phone_number.unwrap();
			let country_code = db::get_phone_country_by_country_code(
				connection,
				&user.backup_phone_country_code.unwrap(),
			)
			.await?
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;

			let phone_number =
				format!("+{}{}", country_code.phone_code, phone_number);

			// send SMS
			sms::send_user_verification_otp(&phone_number, otp)?;
			// panic!("SENDING OTP THROUGH SMS FOR FORGOT PASSWORD IS NOT
			// IMPLEMENTED YET.");
		}
		_ => Error::as_result()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string())?,
	};

	Ok(())
}
