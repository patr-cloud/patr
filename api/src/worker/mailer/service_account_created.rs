use macros::EmailTemplate;
use serde::{Deserialize, Serialize};

use crate::utils::assets::email_images as filters;

/// The email template for the service account created email. This email is
/// sent to the workspace's super admin when a service account is created in
/// their workspace, since its token can do anything its roles allow.
#[derive(Debug, Clone, Serialize, Deserialize, EmailTemplate)]
#[template(
	path = "service-account-created",
	subject = "New service account created in {{ workspace_name }}"
)]
#[serde(rename_all = "camelCase")]
pub struct ServiceAccountCreatedEmail {
	/// The first name of the super admin receiving the email.
	pub first_name: String,
	/// The name of the service account that was created.
	pub service_account_name: String,
	/// The name of the workspace the service account was created in.
	pub workspace_name: String,
	/// The name of whoever created the service account.
	pub created_by: String,
	/// The service account's page on the dashboard (built from the configured
	/// dashboard URL).
	pub service_account_url: String,
}
