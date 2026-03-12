use macros::EmailTemplate;
use serde::{Deserialize, Serialize};

use super::images as filters;

/// The email template for the runner disconnected reminder email. This email is
/// sent to the user when a runner in their workspace is no longer connected to
/// Patr.
#[derive(Debug, Clone, Serialize, Deserialize, EmailTemplate)]
#[template(
	path = "runner-disconnected-reminder",
	subject = "Runner no longer connected to {{workspace_name}}"
)]
#[serde(rename_all = "camelCase")]
pub struct _RunnerDisconnectedReminderEmail {
	/// The username of the user.
	pub username: String,
	/// The name of the runner that disconnected.
	pub runner_name: String,
	/// The name of the workspace the runner belongs to.
	pub workspace_name: String,
	/// The ID of the runner, used to construct the status check URL.
	pub runner_id: String,
}
