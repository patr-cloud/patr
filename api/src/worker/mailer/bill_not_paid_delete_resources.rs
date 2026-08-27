use macros::EmailTemplate;
use serde::{Deserialize, Serialize};

use crate::utils::assets::email_images as filters;

/// The email template for the bill not paid delete resources email. This email
/// is sent to the user when their resources have been deleted due to an unpaid
/// balance on their workspace.
#[derive(Debug, Clone, Serialize, Deserialize, EmailTemplate)]
#[template(
	path = "bill-not-paid-delete-resources",
	subject = "Resources deleted - unpaid balance on {{ workspace_name }}"
)]
#[serde(rename_all = "camelCase")]
pub struct _BillNotPaidDeleteResourcesEmail {
	/// The first name of the user.
	pub first_name: String,
	/// The name of the workspace with the unpaid balance.
	pub workspace_name: String,
	/// The total usage charges that remain unpaid.
	pub total_usage_charges: String,
}
