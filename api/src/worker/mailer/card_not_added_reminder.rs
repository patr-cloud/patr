use macros::EmailTemplate;
use serde::{Deserialize, Serialize};

use crate::utils::assets::email_images as filters;

/// The email template for the card not added reminder email. This email is sent
/// to remind the user to add a payment method to avoid resource deletion.
#[derive(Debug, Clone, Serialize, Deserialize, EmailTemplate)]
#[template(
	path = "card-not-added-reminder",
	subject = "Add a payment method to avoid resource deletion"
)]
#[serde(rename_all = "camelCase")]
pub struct _CardNotAddedReminderEmail {
	/// The username of the recipient.
	pub username: String,
	/// The name of the workspace with unpaid charges.
	pub workspace_name: String,
	/// The total usage charges for the billing period.
	pub total_usage_charges: String,
	/// The deadline by which a card must be added.
	pub deletion_deadline: String,
}
