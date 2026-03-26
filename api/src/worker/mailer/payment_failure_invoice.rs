use macros::EmailTemplate;
use serde::{Deserialize, Serialize};

use crate::utils::assets::email_images as filters;

/// The email template for the payment failure invoice email. This email is sent
/// to the user when a payment fails to process for their workspace.
#[derive(Debug, Clone, Serialize, Deserialize, EmailTemplate)]
#[template(
	path = "payment-failure-invoice",
	subject = "Payment failed for {{ workspace_name }}"
)]
#[serde(rename_all = "camelCase")]
pub struct _PaymentFailureInvoiceEmail {
	/// The username of the user.
	pub username: String,
	/// The first name of the user.
	pub first_name: String,
	/// The name of the workspace.
	pub workspace_name: String,
	/// The amount that was attempted to be charged, in cents.
	pub card_amount_to_be_charged_in_cents: String,
	/// The billing month.
	pub bill_month: String,
	/// The billing year.
	pub bill_year: String,
}
