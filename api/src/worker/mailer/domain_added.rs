use macros::EmailTemplate;
use serde::{Deserialize, Serialize};

use super::images as filters;

/// The email template for the domain added email. This email is sent to the
/// user when a domain is added to their workspace, prompting them to verify
/// DNS.
#[derive(Debug, Clone, Serialize, Deserialize, EmailTemplate)]
#[template(path = "domain-added", subject = "Domain added - verify your DNS")]
#[serde(rename_all = "camelCase")]
pub struct _DomainAddedEmail {
	/// The username of the user.
	pub username: String,
	/// The domain name that was added.
	pub domain_name: String,
	/// The name of the workspace the domain was added to.
	pub workspace_name: String,
	/// The ID of the domain, used for constructing URLs.
	pub domain_id: String,
	/// The number of days before the domain is automatically removed if not
	/// verified.
	pub deadline_limit: String,
}
