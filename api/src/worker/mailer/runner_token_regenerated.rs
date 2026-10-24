use macros::EmailTemplate;
use serde::{Deserialize, Serialize};

use crate::utils::assets::email_images as filters;

/// The email template for the runner token regenerated email. This email is
/// sent to the workspace's super admin when a runner's token is regenerated,
/// since the new token can read every secret in the workspace.
#[derive(Debug, Clone, Serialize, Deserialize, EmailTemplate)]
#[template(
	path = "runner-token-regenerated",
	subject = "Runner token regenerated in {{ workspace_name }}"
)]
#[serde(rename_all = "camelCase")]
pub struct RunnerTokenRegeneratedEmail {
	/// The first name of the super admin receiving the email.
	pub first_name: String,
	/// The name of the runner whose token was regenerated.
	pub runner_name: String,
	/// The name of the workspace the runner belongs to.
	pub workspace_name: String,
	/// The name of whoever regenerated the token.
	pub regenerated_by: String,
	/// The runner's page on the dashboard (built from the configured dashboard
	/// URL).
	pub runner_url: String,
}
