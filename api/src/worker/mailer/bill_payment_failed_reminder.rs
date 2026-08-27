use macros::EmailTemplate;
use serde::{Deserialize, Serialize};

use crate::utils::assets::email_images as filters;

/// The email template for the bill payment failed reminder email. This email is
/// sent to the user when their bill is unpaid and resources are at risk of
/// deletion.
#[derive(Debug, Clone, Serialize, Deserialize, EmailTemplate)]
#[template(
	path = "bill-payment-failed-reminder",
	subject = "Unpaid bill for {{ month }}/{{ year }}"
)]
#[serde(rename_all = "camelCase")]
pub struct _BillPaymentFailedReminderEmail {
	/// The first name of the user.
	pub first_name: String,
	/// The billing month.
	pub month: String,
	/// The billing year.
	pub year: String,
	/// The total charges for the bill.
	pub total_charges: String,
	/// The name of the workspace.
	pub workspace_name: String,
	/// The deadline by which payment must be completed.
	pub deadline: String,
}
