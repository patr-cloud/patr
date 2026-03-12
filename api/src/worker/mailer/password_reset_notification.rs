use macros::EmailTemplate;
use serde::{Deserialize, Serialize};

use super::images as filters;

/// The email template for the password reset notification email. This email is
/// sent to the user when their password has been reset.
#[derive(Debug, Clone, Serialize, Deserialize, EmailTemplate)]
#[template(
	path = "password-reset-notification",
	subject = "Your password has been reset"
)]
#[serde(rename_all = "camelCase")]
pub struct _PasswordResetNotificationEmail {
	/// The username of the user.
	pub username: String,
	/// The first name of the user.
	pub first_name: String,
}
