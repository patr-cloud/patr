use macros::EmailTemplate;
use serde::{Deserialize, Serialize};

use crate::utils::assets::email_images as filters;

/// The email template for the sign-up completed email. This email is sent to
/// the user after their account has been verified and is ready to use.
#[derive(Debug, Clone, Serialize, Deserialize, EmailTemplate)]
#[template(path = "sign-up-completed", subject = "Patr Account Ready")]
#[serde(rename_all = "camelCase")]
pub struct SignUpCompletedEmail {
	/// The username of the user whose account is now ready.
	pub username: String,
}
