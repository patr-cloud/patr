use macros::EmailTemplate;
use serde::{Deserialize, Serialize};

use crate::utils::assets::email_images as filters;

/// The email template for the service account token regenerated email. This
/// email is sent to the workspace's super admin when a service account's token
/// is regenerated, since the old token stops working and the new one can do
/// anything the service account's roles allow.
#[derive(Debug, Clone, Serialize, Deserialize, EmailTemplate)]
#[template(
	path = "service-account-token-regenerated",
	subject = "Service account token regenerated in {{ workspace_name }}"
)]
#[serde(rename_all = "camelCase")]
pub struct ServiceAccountTokenRegeneratedEmail {
	/// The first name of the super admin receiving the email.
	pub first_name: String,
	/// The name of the service account whose token was regenerated.
	pub service_account_name: String,
	/// The name of the workspace the service account belongs to.
	pub workspace_name: String,
	/// The name of whoever regenerated the token.
	pub regenerated_by: String,
	/// The service account's page on the dashboard (built from the configured
	/// dashboard URL).
	pub service_account_url: String,
}
