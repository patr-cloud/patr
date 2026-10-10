use macros::EmailTemplate;
use serde::{Deserialize, Serialize};

use crate::utils::assets::email_images as filters;

/// The email template for the runner created email. This email is sent to the
/// workspace's super admin when a runner is created in their workspace, since
/// the runner's token can read every secret in the workspace.
#[derive(Debug, Clone, Serialize, Deserialize, EmailTemplate)]
#[template(
	path = "runner-created",
	subject = "New runner created in {{ workspace_name }}"
)]
#[serde(rename_all = "camelCase")]
pub struct RunnerCreatedEmail {
	/// The first name of the super admin receiving the email.
	pub first_name: String,
	/// The name of the runner that was created.
	pub runner_name: String,
	/// The name of the workspace the runner was created in.
	pub workspace_name: String,
	/// The name of whoever created the runner.
	pub created_by: String,
	/// The runner's page on the dashboard (built from the configured dashboard
	/// URL).
	pub runner_url: String,
}
