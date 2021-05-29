use eve_rs::AsError;
use sqlx::Transaction;

use crate::{
	db,
	error,
	models::{
		db_mapping::{User, UserLogin},
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

	if phone_number.is_some() {
		// send sms
		log::info!("SENDING SMS ...");
		sms::send_user_verification_otp_sms(country_code, phone_number, otp);
	}
	Ok(())
}
