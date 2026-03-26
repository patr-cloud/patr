use macros::EmailTemplate;
use serde::{Deserialize, Serialize};

use crate::utils::assets::email_images as filters;

/// The email template for the payment success invoice email. This email is sent
/// to the user when their payment has been successfully processed for a
/// workspace billing cycle.
#[derive(Debug, Clone, Serialize, Deserialize, EmailTemplate)]
#[template(
	path = "payment-success-invoice",
	subject = "Payment received for {{ workspace_name }}"
)]
#[serde(rename_all = "camelCase")]
pub struct _PaymentSuccessInvoiceEmail {
	/// The username of the user.
	pub username: String,
	/// The first name of the user.
	pub first_name: String,
	/// The name of the workspace being billed.
	pub workspace_name: String,
	/// The billing month.
	pub bill_month: String,
	/// The billing year.
	pub bill_year: String,
	/// The total charge amount.
	pub total_charge: String,
	/// The amount deducted from the card.
	pub card_amount_deducted: String,
	/// The amount of credits deducted.
	pub credits_deducted: String,
	/// The remaining credits balance.
	pub credits_remaining: String,
}
