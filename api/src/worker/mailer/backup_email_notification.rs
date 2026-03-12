use macros::EmailTemplate;
use serde::{Deserialize, Serialize};

use super::images as filters;

/// The email template for the backup email notification. This email is sent to
/// the user when a recovery email is set for their Patr account.
#[derive(Debug, Clone, Serialize, Deserialize, EmailTemplate)]
#[template(
	path = "backup-email-notification",
	subject = "Recovery email set for your account"
)]
#[serde(rename_all = "camelCase")]
pub struct _BackupEmailNotificationEmail {
	/// The username of the user.
	pub username: String,
	/// The recovery email that was set for the account.
	pub recovery_email: String,
}
