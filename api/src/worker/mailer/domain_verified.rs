use macros::EmailTemplate;
use serde::{Deserialize, Serialize};

use super::images as filters;

/// The email template for the domain verified email. This email is sent to the
/// user when their domain has been successfully verified and is ready to use.
#[derive(Debug, Clone, Serialize, Deserialize, EmailTemplate)]
#[template(path = "domain-verified", subject = "Domain verified")]
#[serde(rename_all = "camelCase")]
pub struct _DomainVerifiedEmail {
	/// The username of the user who owns the domain.
	pub username: String,
	/// The domain name that has been verified.
	pub domain_name: String,
}
