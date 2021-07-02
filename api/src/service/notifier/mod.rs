use eve_rs::AsError;

use crate::{
	db,
	error,
	models::db_mapping::{PreferredRecoveryOption, User, UserToSignUp},
	utils::Error,
	Database,
};

mod email;
mod sms;

pub use email::*;
pub use sms::*;

/// # Description
/// This function is used to send sign_in complete notification
///
/// # Arguments
/// * `welcome_email` - an Option<String> containing either String which has
///   user's personal or
/// organisation email to send a welcome notification to or `None`
/// * `backup_email` - an Option<String> containing either String which has
///   user's backup email
/// to send a verification email to or `None`
/// * `backup_phone_number` - an Option<String> containing either String which
///   has user's backup phone
/// number to send a verification sms to or `None`
///
/// # Returns
/// This function returns `Result<(), Error>` containing an empty response or an
/// error
pub async fn send_sign_up_complete_notification(
	welcome_email: Option<String>,
	backup_email: Option<String>,
	backup_phone_number: Option<String>,
) -> Result<(), Error> {
	if let Some(welcome_email) = welcome_email {
		email::send_sign_up_completed_email(welcome_email.parse()?).await?;
	}

	if let Some(backup_email) = backup_email {
		email::send_backup_registration_mail(backup_email.parse()?).await?;
	}

	if let Some(phone_number) = backup_phone_number {
		sms::send_backup_registration_sms(&phone_number).await?;
	}
	Ok(())
}

/// # Description
/// This function is used to send otp to user's email for sign-up
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `user` - an object of type [`UserToSignUp`]
/// * `otp` - a string containing otp to be sent
///
/// # Returns
/// This function returns `Result<(), Error>` containing an empty response or an
/// error
///
/// [`Transaction`]: Transaction
pub async fn send_user_sign_up_otp(
	connection: &mut <Database as sqlx::Database>::Connection,
	user: UserToSignUp,
	otp: &str,
) -> Result<(), Error> {
	// chcek if email is given as a backup option
	if let Some((backup_email_domain_id, backup_email_local)) = user
		.backup_email_domain_id
		.as_ref()
		.zip(user.backup_email_local.as_ref())
	{
		let email = get_user_email(
			connection,
			&backup_email_domain_id,
			&backup_email_local,
		)
		.await?;

		email::send_user_verification_otp(email.parse()?, otp).await?;
	} else if let Some((phone_country_code, phone_number)) = user
		.backup_phone_country_code
		.as_ref()
		.zip(user.backup_phone_number.as_ref())
	{
		// check if phone number is given as a backup
		let phone_number = get_user_phone_number(
			connection,
			&phone_country_code,
			&phone_number,
		)
		.await?;

		sms::send_user_verification_otp(&phone_number, otp).await?;
	}

	Ok(())
}

/// # Description
/// This function is used to send the given otp to all the backup options
/// available for the given user.
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `user` - an object of type [`User`] containing all details of user
///
/// # Returns
/// This function returns `Result<(), Error>` containing an empty response or an
/// error
///
/// [`Transaction`]: Transaction
pub async fn send_password_changed_notification(
	connection: &mut <Database as sqlx::Database>::Connection,
	user: User,
) -> Result<(), Error> {
	// chcek if email is given as a backup option
	if let Some((backup_email_domain_id, backup_email_local)) = user
		.backup_email_domain_id
		.as_ref()
		.zip(user.backup_email_local.as_ref())
	{
		let email = get_user_email(
			connection,
			&backup_email_domain_id,
			&backup_email_local,
		)
		.await?;

		email::send_password_changed_notification(email.parse()?).await?;
	}

	// check if phone number is given as a backup
	if let Some((phone_country_code, phone_number)) = user
		.backup_phone_country_code
		.as_ref()
		.zip(user.backup_phone_number.as_ref())
	{
		let phone_number = get_user_phone_number(
			connection,
			&phone_country_code,
			&phone_number,
		)
		.await?;

		sms::send_password_changed_notification(&phone_number).await?;
	}
	Ok(())
}

/// # Description
/// This function is used to sent user reset password notification
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `user` - an object of type [`User`] containing all details of user
///
/// # Returns
/// This function returns `Result<(), Error>` containing an empty response or an
/// error
///
/// [`Transaction`]: Transaction
pub async fn send_user_reset_password_notification(
	connection: &mut <Database as sqlx::Database>::Connection,
	user: User,
) -> Result<(), Error> {
	if let Some((phone_country_code, phone_number)) = user
		.backup_phone_country_code
		.as_ref()
		.zip(user.backup_phone_number.as_ref())
	{
		let phone_number = get_user_phone_number(
			connection,
			&phone_country_code,
			&phone_number,
		)
		.await?;

		sms::send_user_reset_password_notification(&phone_number).await?;
	}

	if let Some((backup_email_domain_id, backup_email_local)) = user
		.backup_email_domain_id
		.as_ref()
		.zip(user.backup_email_local.as_ref())
	{
		let email = get_user_email(
			connection,
			&backup_email_domain_id,
			&backup_email_local,
		)
		.await?;

		email::send_user_reset_password_notification(email.parse()?).await?;
	}
	Ok(())
}

/// # Description
/// This function is used to send otp incase the user forgets the password
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `user` - an object of type [`User`] containing all details of user
/// * `recovery_option` - an object of type [`PreferredRecoveryOption`]
/// * `otp` - a string containing otp to be sent to user
///  
/// # Returns
/// This function returns `Result<(), Error>` containing an empty response or an
/// error
///
/// [`Transaction`]: Transaction
pub async fn send_forgot_password_otp(
	connection: &mut <Database as sqlx::Database>::Connection,
	user: User,
	recovery_option: PreferredRecoveryOption,
	otp: &str,
) -> Result<(), Error> {
	// match on the recovery type
	match recovery_option {
		PreferredRecoveryOption::BackupEmail => {
			let email = get_user_email(
				connection,
				user.backup_email_domain_id
					.as_ref()
					.status(400)
					.body(error!(WRONG_PARAMETERS).to_string())?,
				&user
					.backup_email_local
					.status(400)
					.body(error!(WRONG_PARAMETERS).to_string())?,
			)
			.await?;

			// send email
			email::send_forgot_password_otp(email.parse()?, otp).await?;
		}
		PreferredRecoveryOption::BackupPhoneNumber => {
			let phone_number = get_user_phone_number(
				connection,
				user.backup_phone_country_code
					.as_ref()
					.status(400)
					.body(error!(WRONG_PARAMETERS).to_string())?,
				&user
					.backup_phone_number
					.status(400)
					.body(error!(WRONG_PARAMETERS).to_string())?,
			)
			.await?;
			// send SMS
			sms::send_forgot_password_otp(&phone_number, otp).await?;
		}
	};

	Ok(())
}

/// # Description
/// This function is used to get the user's email address
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `domain_id` - An unsigned 8 bit integer array containing id of
/// organisation domain
/// * `email_string` - a string containing user's email_local
///  
/// # Returns
/// This function returns `Result<String, Error>` containing user's email
/// address or an error
///
/// [`Transaction`]: Transaction
async fn get_user_email(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_id: &[u8],
	email_string: &str,
) -> Result<String, Error> {
	let domain = db::get_personal_domain_by_id(connection, domain_id)
		.await?
		.status(500)?;
	let email = format!("{}@{}", email_string, domain.name);
	Ok(email)
}

/// # Description
/// This function is used to get the user's complete phone number
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `country_code` - a string containing 2 letter country code
/// * `phone_number` - a string containing user's phone number
///  
/// # Returns
/// This function returns `Result<String, Error>` containing user's complete
/// phone number or an error
///
/// [`Transaction`]: Transaction
async fn get_user_phone_number(
	connection: &mut <Database as sqlx::Database>::Connection,
	country_code: &str,
	phone_number: &str,
) -> Result<String, Error> {
	let country_code =
		db::get_phone_country_by_country_code(connection, country_code)
			.await?
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;

	let phone_number = format!("+{}{}", country_code.phone_code, phone_number);
	Ok(phone_number)
}
