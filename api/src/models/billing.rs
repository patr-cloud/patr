use std::collections::HashMap;

use api_models::utils::{True, Uuid};
use serde::{Deserialize, Serialize};

use crate::db::{DomainPlan, StaticSitePlan};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentBill {
	pub deployment_id: Uuid,
	pub deployment_name: String,
	pub bill_items: Vec<DeploymentBillItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentBillItem {
	pub machine_type: (u16, u32), // CPU, RAM
	pub num_instances: u32,
	pub hours: u64,
	pub amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseBill {
	pub database_id: Uuid,
	pub database_name: String,
	pub hours: u64,
	pub amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StaticSiteBill {
	pub hours: u64,
	pub amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedUrlBill {
	pub url_count: u64,
	pub hours: u64,
	pub amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerRepositoryBill {
	pub storage: u64,
	pub hours: u64,
	pub amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DomainBill {
	pub hours: u64,
	pub amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretsBill {
	pub secrets_count: u64,
	pub hours: u64,
	pub amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ThreeDSecureUsage {
	pub supported: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct PaymentIntentObject {
	pub id: String,
	pub amount: Option<f64>,
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
	pub confirm: True,
	pub off_session: bool,
	pub description: String,
	pub customer: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub payment_method: Option<String>,
	#[serde(rename = "payment_method_types[]")]
	pub payment_method_types: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub setup_future_usage: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct StripeAddress {
	#[serde(rename = "address[city]")]
	pub city: String,
	#[serde(rename = "address[country]")]
	pub country: String,
	#[serde(rename = "address[line1]")]
	pub line1: String,
	#[serde(rename = "address[line2]")]
	pub line2: Option<String>,
	#[serde(rename = "address[postal_code]")]
	pub postal_code: String,
	#[serde(rename = "address[state]")]
	pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoicePdf {
	pub workspace_name: String,
	pub deployment_usages: HashMap<Uuid, DeploymentBill>,
	pub database_usages: HashMap<Uuid, DatabaseBill>,
	pub static_sites_usages: HashMap<StaticSitePlan, StaticSiteBill>,
	pub managed_url_usages: HashMap<u64, ManagedUrlBill>,
	pub docker_repository_usages: Vec<DockerRepositoryBill>,
	pub domains_usages: HashMap<DomainPlan, DomainBill>,
	pub secrets_usages: HashMap<u64, SecretsBill>,
	pub total_bill: f64,
	pub month: String,
	pub year: i32,
}
