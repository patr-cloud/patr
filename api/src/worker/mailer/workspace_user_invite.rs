use macros::EmailTemplate;
use serde::{Deserialize, Serialize};

use crate::utils::assets::email_images as filters;

/// The email template for the workspace invite email. This is sent to an email
/// address that has been invited to join a workspace, and contains a link to
/// accept the invite.
#[derive(Debug, Clone, Serialize, Deserialize, EmailTemplate)]
#[template(
	path = "workspace-invite",
	subject = "You've been invited to join {{ workspace_name }} | Patr"
)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceInviteEmail {
	/// The name of the workspace the invitee has been invited to.
	pub workspace_name: String,
	/// The name of the user who sent the invite.
	pub invited_by: String,
	/// The full accept link (built from the configured dashboard URL).
	pub accept_url: String,
	/// The validity duration of the invite, in a human-readable format.
	pub expiry: String,
}
