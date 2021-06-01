use core::panic;

use crate::utils::Error;

// TODO: implement this
pub fn send_user_verification_otp(
	_email: &str,
	_otp: &str,
) -> Result<(), Error> {
	log::error!("SENDING OTP EMAIL ...");
	panic!("Sending email in notifier is not implemented yet.");
	Ok(())
}

pub fn send_user_reset_password_notification(
	_email: &str,
) -> Result<(), Error> {
	log::error!("SENDING RESET PASSWORD NOTIFICATION EMAIL ...");
	panic!("Sending email in notifier is not implemented yet.");
	Ok(())
}

pub fn send_password_changed_notification(_email: &str) -> Result<(), Error> {
	log::error!("SENDING PASSWORD CHANGE EMAIL ...");
	panic!("Sending email in notifier is not implemented yet.");
	Ok(())
}

pub fn send_sign_up_completed_email(_email: &str) -> Result<(), Error> {
	log::error!("SENDING WELCOME EMAIL ...");
	panic!("Sending email in notifier is not implemented yet.");
	Ok(())
}

pub fn send_backup_registration_mail(_email: &str) -> Result<(), Error> {
	log::error!("SENDING BACKUP REGISTRATION EMAIL ...");
	panic!("Sending email in notifier is not implemented yet.");
	Ok(())
}
