use macros::EmailTemplate;
use serde::{Deserialize, Serialize};

use crate::utils::assets::email_images as filters;

/// The email template for the unverified domain delete email. This email is
/// sent to the user when their domain is removed due to not being verified
/// within the deadline.
#[derive(Debug, Clone, Serialize, Deserialize, EmailTemplate)]
#[template(
	path = "unverified-domain-delete",
	subject = "Unverified domain removed"
)]
#[serde(rename_all = "camelCase")]
pub struct _UnverifiedDomainDeleteEmail {
	/// The username of the user.
	pub username: String,
	/// The domain name that was removed.
	pub domain_name: String,
	/// The deadline limit after which the domain was removed (e.g. "30 days").
	pub deadline_limit: String,
}
