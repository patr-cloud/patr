use macros::EmailTemplate;
use serde::{Deserialize, Serialize};

use crate::utils::assets::email_images as filters;

/// The email template for the forgot password email. This email is sent to the
/// user when they request a password reset, and contains an OTP for
/// verification.
#[derive(Debug, Clone, Serialize, Deserialize, EmailTemplate)]
#[template(path = "forgot-password", subject = "Your password reset code")]
#[serde(rename_all = "camelCase")]
pub struct ForgotPasswordEmail {
	/// The first name of the user who requested the password reset.
	pub first_name: String,
	/// The email address of the user who requested the password reset.
	pub email: String,
	/// The OTP code to be sent to the user for password reset verification.
	pub otp_code: String,
	/// The expiry time of the OTP, in a human-readable format.
	pub otp_expiry: String,
}
