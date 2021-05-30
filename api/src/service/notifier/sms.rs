use core::panic;

use eve_rs::AsError;
use sqlx::Transaction;

use crate::{
	db, error,
	models::{
		db_mapping::{User, UserLogin},
		rbac, AccessTokenData, ExposedUserData,
	},
	service::{self, get_refresh_token_expiry},
	utils::Error,
	Database,
};

//TODO: implement this function
pub fn send_user_verification_otp(
	country_code: &str,
	phone_number: &str,
	otp: &str,
) -> Result<(), Error> {
	log::error!("SENDING MESSAGE...");
	panic!("sending SMS with phone number is not implemented yet.");
	Ok(())
}

pub fn send_user_reset_password_notification(
	country_code: &str,
	phone_number: &str,
) -> Result<(), Error> {
	log::error!("SENDING PASSWORD RESET NOTIFICATION");
	panic!("sending SMS with phone number is not implenented yet.");
	Ok(())
}
