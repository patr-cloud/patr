use macros::EmailTemplate;
use serde::{Deserialize, Serialize};

use super::images as filters;

/// The email template for the password changed notification email. This email
/// is sent to the user when their password has been changed.
#[derive(Debug, Clone, Serialize, Deserialize, EmailTemplate)]
#[template(
	path = "password-changed-notification",
	subject = "Your password was changed"
)]
#[serde(rename_all = "camelCase")]
pub struct _PasswordChangedNotificationEmail {
	/// The username of the user whose password was changed.
	pub username: String,
	/// The first name of the user, used in the greeting.
	pub first_name: String,
}
