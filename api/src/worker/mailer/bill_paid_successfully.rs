use macros::EmailTemplate;
use serde::{Deserialize, Serialize};

use super::images as filters;

/// The email template for the bill paid successfully email. This email is sent
/// to the user when their payment has been processed successfully.
#[derive(Debug, Clone, Serialize, Deserialize, EmailTemplate)]
#[template(path = "bill-paid-successfully", subject = "Payment received")]
#[serde(rename_all = "camelCase")]
pub struct _BillPaidSuccessfullyEmail {
	/// The username of the user.
	pub username: String,
	/// The payment amount, formatted as a string (e.g. "$12.00").
	pub amount: String,
	/// The billing month (e.g. "January").
	pub billing_month: String,
	/// The billing year (e.g. "2026").
	pub billing_year: String,
	/// The name of the workspace.
	pub workspace_name: String,
}
