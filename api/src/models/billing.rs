use api_models::models::workspace::billing::{
	CardFundingType,
	CardNetworks,
	PaymentMethod,
	StripeCustomer,
	ThreeDSecureUsage,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentMethodStatus {
	RequiresPaymentMethod,
	RequiresConfirmation,
	RequiresAction,
	Processing,
	Canceled,
	Succeeded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentMethodUsage {
	OnSession,
	OffSession,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Card {
	pub brand: String,
	pub country: String,
	pub exp_month: u32,
	pub exp_year: u32,
	pub fingerprint: String,
	pub funding: CardFundingType,
	pub last4: String,
	pub networks: CardNetworks,
	pub three_d_secure_usage: ThreeDSecureUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LastSetupError {
	pub code: String,
	pub decline_code: String,
	pub doc_url: String,
	pub message: String,
	pub param: String,
	pub payment_method: Option<PaymentMethod>,
	pub payment_method_type: String,
	pub r#type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PaymentIntentObject {
	pub id: String,
	pub client_secret: String,
	pub customer: Option<StripeCustomer>,
	pub description: Option<String>,
	pub last_setup_error: Option<LastSetupError>,
	pub payment_method: Option<PaymentMethod>,
	pub payment_method_types: Vec<String>,
	pub status: PaymentMethodStatus,
	pub usage: PaymentMethodUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PaymentIntent {
	pub amount: f64,
	pub currency: String,
	pub payment_method: String,
	// TODO: find a method to send payment_method_types array
	#[serde(rename = "payment_method_types[]")]
	pub payment_method_types: String,
	pub customer: String,
	// TODO: add field for payment method
}

pub struct ProductLimits {
	pub deployment: i32,
	pub static_site: i32,
	pub managed_database: i32,
	pub managed_url: i32,
	pub domain: i32,
	pub secret: i32,
}
