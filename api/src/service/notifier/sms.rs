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

//TODO: implement this function
pub fn send_user_verification_otp(
	country_code: &str,
	phone_number: &str,
	otp: &str,
) {
	log::error!("sending sms...");
	panic!("sending SMS with phone number is not implemented yet.")
}
