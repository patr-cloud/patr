use macros::EmailTemplate;
use serde::{Deserialize, Serialize};

use super::images as filters;

/// The email template for the delete resource email. This email is sent to the
/// user when a resource is deleted from a workspace.
#[derive(Debug, Clone, Serialize, Deserialize, EmailTemplate)]
#[template(
	path = "delete-resource",
	subject = "Resource deleted in {{ workspace_name }}"
)]
#[serde(rename_all = "camelCase")]
pub struct _DeleteResourceEmail {
	/// The username of the user receiving the email.
	pub username: String,
	/// The type of the resource that was deleted.
	pub resource_type: String,
	/// The name of the resource that was deleted.
	pub resource_name: String,
	/// The name of the workspace from which the resource was deleted.
	pub workspace_name: String,
	/// The name or identifier of the user who deleted the resource.
	pub deleted_by: String,
}
