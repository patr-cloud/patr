use macros::EmailTemplate;
use serde::{Deserialize, Serialize};

use super::images as filters;

/// The email template for the domain not verified reminder email. This email is
/// sent to the user when a domain added to their workspace hasn't been verified
/// yet, reminding them to complete DNS verification.
#[derive(Debug, Clone, Serialize, Deserialize, EmailTemplate)]
#[template(
	path = "domain-not-verified-reminder",
	subject = "Your domain is still unverified"
)]
#[serde(rename_all = "camelCase")]
pub struct _DomainNotVerifiedReminderEmail {
	/// The username of the user.
	pub username: String,
	/// The domain name that hasn't been verified.
	pub domain_name: String,
	/// The name of the workspace the domain was added to.
	pub workspace_name: String,
	/// The ID of the domain, used to construct the verification link.
	pub domain_id: String,
}
