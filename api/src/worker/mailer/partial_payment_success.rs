use macros::EmailTemplate;
use serde::{Deserialize, Serialize};

use crate::utils::assets::email_images as filters;

/// The email template for the partial payment success email. This email is sent
/// to the user when a partial payment has been received toward their bill.
#[derive(Debug, Clone, Serialize, Deserialize, EmailTemplate)]
#[template(path = "partial-payment-success", subject = "Partial payment received")]
#[serde(rename_all = "camelCase")]
pub struct _PartialPaymentSuccessEmail {
	/// The username of the recipient.
	pub username: String,
	/// The amount that was paid.
	pub amount_paid: String,
	/// The name of the workspace the bill is for.
	pub workspace_name: String,
	/// The total bill amount.
	pub total_bill_amount: String,
	/// The remaining balance due.
	pub remaining_balance: String,
	/// Whether the balance has been fully cleared (e.g. via credits).
	pub balance_cleared: bool,
	/// The amount of credits applied, if any.
	pub credits_applied: String,
}
