use macros::EmailTemplate;
use serde::{Deserialize, Serialize};

use super::images as filters;

/// The email template for the add new email notification. This email is sent to
/// the user when they add a new email address to their account, and contains an
/// OTP for verification.
#[derive(Debug, Clone, Serialize, Deserialize, EmailTemplate)]
#[template(
	path = "add-new-email-notification",
	subject = "Verify your new email address | Patr"
)]
#[serde(rename_all = "camelCase")]
pub struct _AddNewEmailNotificationEmail {
	/// The username of the user.
	pub username: String,
	/// The first name of the user.
	pub first_name: String,
	/// The email address to be verified.
	pub email: String,
	/// The OTP to be sent to the user for verification.
	pub otp: String,
	/// The expiry time of the OTP, in a human-readable format.
	pub otp_expiry: String,
}
