use macros::EmailTemplate;
use serde::{Deserialize, Serialize};

use crate::utils::assets::email_images as filters;

/// The email template for the purchase credits success email. This email is
/// sent to the user when credits have been successfully added to their
/// workspace.
#[derive(Debug, Clone, Serialize, Deserialize, EmailTemplate)]
#[template(
	path = "purchase-credits-success",
	subject = "Credits added to {{ workspace_name }}"
)]
#[serde(rename_all = "camelCase")]
pub struct _PurchaseCreditsSuccessEmail {
	/// The first name of the user.
	pub first_name: String,
	/// The amount of credits added.
	pub credit_amount: String,
	/// The name of the workspace the credits were added to.
	pub workspace_name: String,
}
