pub mod ci;
pub mod deployment;
pub mod error;
pub mod rabbitmq;
pub mod rbac;

mod access_token_data;
mod auditlog;
mod docker_registry;
mod email_template;
#[cfg(feature = "sample-data")]
mod sample_data;
mod twilio_sms;

use serde::{Deserialize, Serialize};

#[cfg(feature = "sample-data")]
pub use self::sample_data::*;
pub use self::{
	access_token_data::*,
	auditlog::*,
	docker_registry::*,
	email_template::*,
	twilio_sms::*,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ThreeDSecureUsage {
	pub supported: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct PaymentIntentObject {
	pub id: String,
	pub amount: f64,
	pub automatic_payment_method: Option<AutomaticPaymentMethods>,
	pub client_secret: String,
	pub customer: Option<String>,
	pub description: Option<String>,
	pub last_setup_error: Option<LastSetupError>,
	pub payment_method: Option<String>,
	pub payment_method_types: Vec<String>,
	pub status: Option<PaymentMethodStatus>,
	pub usage: Option<PaymentMethodUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PaymentMethodStatus {
	RequiresPaymentMethod,
	RequiresConfirmation,
	RequiresAction,
	Processing,
	Canceled,
	Succeeded,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PaymentMethodUsage {
	OnSession,
	OffSession,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct Card {
	pub brand: String,
	pub country: String,
	pub exp_month: u32,
	pub exp_year: u32,
	pub fingerprint: String,
	pub funding: CardFundingType,
	pub last4: String,
	pub three_d_secure_usage: ThreeDSecureUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct PaymentMethod {
	pub id: String,
	pub customer: String,
	pub r#type: StripePaymentMethodType,
	pub card: Option<Card>,
	//TODO: Add other payment methods
	pub created: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StripePaymentMethodType {
	AcssDebit,
	Affirm,
	AfterpayClearpay,
	Alipay,
	// Add more payment methods types layers
	// refer this: https://stripe.com/docs/api/payment_methods/object#payment_method_object-type
	Card,
	CardPresent,
	UsBankAccount,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct AutomaticPaymentMethods {
	pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct PaymentIntent {
	pub amount: u64,
	pub currency: String,
	pub description: String,
	pub customer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CardFundingType {
	Debit,
	Credit,
	Prepaid,
	Unknown,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct CardNetworks {
	pub available: Vec<String>,
	pub preferred: Option<String>,
}
