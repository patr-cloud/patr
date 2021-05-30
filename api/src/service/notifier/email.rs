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
	utils::{
		constants::ResourceOwnerType, get_current_time_millis,
		settings::Settings, validator, Error,
	},
	Database,
};

// TODO: implement this
pub fn send_user_verification_otp(email: &str, otp: &str) -> Result<(), Error> {
	log::error!("SENDING EMAIL ...");
	panic!("Sending email in notifier is not implemented yet.");
	Ok(())
}

pub fn send_user_reset_password_notification(email: &str) -> Result<(), Error> {
	log::error!("SENDING EMAIL ...");
	panic!("Sending email in notifier is not implemented yet.");
	Ok(())
}
