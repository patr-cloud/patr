use macros::EmailTemplate;
use serde::{Deserialize, Serialize};

/// The email template for the user sign up email. This email is sent to the
/// user when they sign up, and contains an OTP for verification.
#[derive(Debug, Clone, Serialize, Deserialize, EmailTemplate)]
#[template(path = "user-sign-up", subject = "Verify your account | Patr")]
#[serde(rename_all = "camelCase")]
pub struct UserSignUpEmail {
	/// The username of the user who signed up.
	pub username: String,
	/// The OTP to be sent to the user for verification.
	pub otp: String,
	/// The expiry time of the OTP, in a human-readable format.
	pub otp_expiry: String,
}
