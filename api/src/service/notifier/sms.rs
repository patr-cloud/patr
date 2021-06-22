use crate::{service::notifier, utils::Error, Database};

pub async fn send_user_verification_otp(
	phone_number: &str,
	otp: &str,
) -> Result<(), Error> {
	send_sms(
		phone_number,
		format!(
			"{}{}{}",
			"Thank you for signing up for Vicara's deployment tool. ",
			"The OTP to register your account is: ",
			otp
		),
	)
	.await
}

pub async fn send_password_changed_notification(
	phone_number: &str,
) -> Result<(), Error> {
	send_sms(
		phone_number,
		format!(
			"{}{}{}",
			"Your account password for Vicara's ",
			"deployment tool has recently been updated. ",
			"If this was not you, please login or contact support immediately"
		),
	)
	.await
}

pub async fn send_forgot_password_otp(
	phone_number: &str,
	otp: &str,
) -> Result<(), Error> {
	send_sms(
		phone_number,
		format!(
			"{}{}{}{}{}",
			"We received a password reset request from your Deployment Tool ",
			"account. Your OTP to reset the same is: ",
			otp,
			"If this process was not initiated by you, ",
			"please contact support immediately."
		),
	)
	.await
}

pub async fn send_user_reset_password_notification(
	phone_number: &str,
) -> Result<(), Error> {
	send_sms(
		phone_number,
		format!(
			"{}{}{}",
			"Your account password for Vicara's ",
			"deployment tool has recently been reset. ",
			"If this was not you, please contact support immediately"
		),
	)
	.await
}

pub async fn send_backup_registration_sms(
	phone_number: &str,
) -> Result<(), Error> {
	send_sms(
		phone_number,
		format!(
			"{}{}{}",
			"This phone number is now set as the backup phone ",
			"for the Vicara's deployment tool. ",
			"If this was not you, please login or contact support immediately"
		),
	)
	.await
}

pub async fn send_phone_number_verification_otp(
	connection: &mut <Database as sqlx::Database>::Connection,
	country_code: &str,
	phone_number: &str,
	otp: &str,
) -> Result<(), Error> {
	let phone_number =
		notifier::get_user_phone_number(connection, country_code, phone_number)
			.await?;

	send_sms(
		&phone_number,
		format!(
			"{}{}{}{}",
			"We recieved a new phone number addition request from your account.",
			" Please enter this otp to complete the process: ",
			otp,
			" If this was not you, please contact support immediately."
		),
	)
	.await
}

#[cfg(not(debug_assertions))]
async fn send_sms(to_number: &str, body: String) -> Result<(), Error> {
	use reqwest::Client;

	use crate::{
		error,
		models::{SmsRequest, SmsResponse},
		service,
	};

	let config = service::get_config();
	let client = Client::new();
	let response = client
		.post(format!(
			"https://{}:{}@api.twilio.com/2010-04-01/Accounts/{}/Messages.json",
			config.twilio.username,
			config.twilio.access_token,
			config.twilio.username
		))
		.json(&SmsRequest {
			body,
			from: config.twilio.from_number.clone(),
			to: to_number.to_string(),
		})
		.send()
		.await?
		.json::<SmsResponse>()
		.await?;

	if response.status == "queued" {
		log::info!(target: "sms", "Sms to {} sent successfully!", to_number);
		Ok(())
	} else if response.status == "undelivered" {
		Err(Error::empty()
			.status(400)
			.body(error!(INVALID_PHONE_NUMBER).to_string()))
	} else {
		Err(Error::empty()
			.status(500)
			.body(error!(SERVER_ERROR).to_string()))
	}
}

#[cfg(debug_assertions)]
async fn send_sms(to_number: &str, body: String) -> Result<(), Error> {
	log::trace!("Sending sms with body: {:#?} to {}", body, to_number);
	Ok(())
}
