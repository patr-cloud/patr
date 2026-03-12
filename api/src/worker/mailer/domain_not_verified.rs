use macros::EmailTemplate;
use serde::{Deserialize, Serialize};

use super::images as filters;

/// The email template for the domain not verified email. This email is sent to
/// the user when their domain is no longer pointing to Patr and needs to be
/// re-verified.
#[derive(Debug, Clone, Serialize, Deserialize, EmailTemplate)]
#[template(
	path = "domain-not-verified",
	subject = "Domain no longer pointing to Patr"
)]
#[serde(rename_all = "camelCase")]
pub struct _DomainNotVerifiedEmail {
	/// The username of the user.
	pub username: String,
	/// The domain name that is no longer verified.
	pub domain_name: String,
	/// The number of days the user has to re-verify the domain.
	pub deadline_days: String,
	/// The ID of the domain, used for constructing the re-verify link.
	pub domain_id: String,
}
