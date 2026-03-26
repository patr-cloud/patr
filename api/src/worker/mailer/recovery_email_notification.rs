use macros::EmailTemplate;
use serde::{Deserialize, Serialize};

use crate::utils::assets::email_images as filters;

/// The email template for the recovery email notification. This email is sent
/// when a recovery email address is added to a user's Patr account.
#[derive(Debug, Clone, Serialize, Deserialize, EmailTemplate)]
#[template(
	path = "recovery-email-notification",
	subject = "Recovery email set for your Patr account"
)]
#[serde(rename_all = "camelCase")]
pub struct _RecoveryEmailNotificationEmail {
	/// The username of the user.
	pub username: String,
	/// The first name of the user.
	pub first_name: String,
}
