use serde_json::Value;

use crate::{
	models::{error, SmsRequest},
	utils::settings::Settings,
};

#[allow(dead_code)]
pub async fn send_otp_sms(
	config: Settings,
	to_number: String,
	otp: String,
) -> Result<(), String> {
	let request = surf::post(format!(
		"https://{}:{}@api.twilio.com/2010-04-01/Accounts/{}/Messages.json",
		config.twilio.username,
		config.twilio.access_token,
		config.twilio.username
	))
	.body(
		serde_json::to_value(SmsRequest {
			body: format!(
				"Welcome to Vicara! The OTP to verify your account is {}",
				otp
			),
			from: config.twilio.from_number,
			to: to_number.clone(),
		})
		.map_err(|err| err.to_string())?,
	)
	.recv_string();

	// Send the email
	match request.await {
		Ok(data) => {
			let result = serde_json::from_str(&data);
			let data: Value = if let Ok(data) = result {
				data
			} else {
				return Err(String::from(error::id::SERVER_ERROR));
			};
			let status = if let Some(Value::String(status)) = data.get("status")
			{
				status
			} else {
				return Err(String::from(error::id::SERVER_ERROR));
			};
			if status == "queued" {
				log::info!(target: "sms", "Sign up confirmation sms to {} sent successfully!", to_number);
				Ok(())
			} else if status == "undelivered" {
				Err(String::from(error::id::INVALID_PHONE_NUMBER))
			} else {
				Err(String::from(error::id::SERVER_ERROR))
			}
		}
		Err(err) => {
			log::error!(target: "sms", "Could not send sms to {}: {:#?}", to_number, err);
			Err(String::from(error::id::SERVER_ERROR))
		}
	}
}
