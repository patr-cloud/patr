use macros::EmailTemplate;
use serde::{Deserialize, Serialize};

use super::images as filters;

/// The email template for the bill paid using credits email. This email is sent
/// to the user when their bill has been fully or partially paid using Patr
/// credits.
#[derive(Debug, Clone, Serialize, Deserialize, EmailTemplate)]
#[template(
	path = "bill-paid-using-credits",
	subject = "Payment applied from your Patr credits"
)]
#[serde(rename_all = "camelCase")]
pub struct _BillPaidUsingCreditsEmail {
	/// The username of the user.
	pub username: String,
	/// The total bill amount.
	pub total_bill: String,
	/// The name of the workspace the bill is for.
	pub workspace_name: String,
	/// Whether the bill was fully paid using credits.
	pub fully_paid: bool,
	/// The remaining credit balance after payment (used when fully paid).
	pub remaining_credits: String,
	/// The remaining amount due (used when partially paid).
	pub reduced_bill: String,
}
