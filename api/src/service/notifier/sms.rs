use core::panic;

use crate::{db, error, utils::Error};

//TODO: implement this function
pub fn send_user_verification_otp(
	_phone_number: &str,
	_otp: &str,
) -> Result<(), Error> {
	log::error!("SENDING OTP MESSAGE...");
	panic!("sending SMS with phone number is not implemented yet.");
	Ok(())
}

pub fn send_password_changed_notification(
	_phone_number: &str,
) -> Result<(), Error> {
	log::error!("SENDING PASSWORD CHANGE MESSAGE...");
	panic!("sending SMS with phone number is not implemented yet.");
	Ok(())
}

pub fn send_user_reset_password_notification(
	_phone_number: &str,
) -> Result<(), Error> {
	log::error!("SENDING PASSWORD RESET MESSAGE");
	panic!("sending SMS with phone number is not implenented yet.");
	Ok(())
}

pub fn send_backup_registration_sms(_phone_number: &str) -> Result<(), Error> {
	log::error!("SENDING BACKUP MESSAGE...");
	panic!("sending SMS with phone number is not implemented yet.");
	Ok(())
}
