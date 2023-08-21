use core::fmt;
use std::ops::Add;

use chrono::Utc;
use serde::{Deserialize, Serialize};

mod add_billing_address;
mod add_credits;
mod add_payment_method;
mod confirm_credits;
mod confirm_payment;
mod confirm_payment_method;
mod delete_billing_address;
mod delete_payment_method;
mod get_bill_breakdown;
mod get_billing_address;
mod get_current_usage;
mod get_payment_method;
mod get_transactions;
mod make_payment;
mod set_primary_card;
mod update_billing_address;

pub use self::{
	add_billing_address::*,
	add_credits::*,
	add_payment_method::*,
	confirm_credits::*,
	confirm_payment::*,
	confirm_payment_method::*,
	delete_billing_address::*,
	delete_payment_method::*,
	get_bill_breakdown::*,
	get_billing_address::*,
	get_current_usage::*,
	get_payment_method::*,
	get_transactions::*,
	make_payment::*,
	set_primary_card::*,
	update_billing_address::*,
};
use super::infrastructure::DeploymentMachineType;
use crate::utils::{DateTime, Uuid};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Address {
	pub first_name: String,
	pub last_name: String,
	pub address_line_1: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub address_line_2: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub address_line_3: Option<String>,
	pub city: String,
	pub state: String,
	pub zip: String,
	pub country: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Card {
	pub brand: String,
	pub country: String,
	#[serde(alias = "exp_month")]
	pub exp_month: u32,
	#[serde(alias = "exp_year")]
	pub exp_year: u32,
	pub funding: CardFundingType,
	pub last4: String,
}

#[cfg(feature = "server")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[serde(rename_all = "camelCase")]
#[sqlx(type_name = "PAYMENT_STATUS", rename_all = "lowercase")]
pub enum PaymentStatus {
	Success,
	Pending,
	Failed,
}

#[cfg(not(feature = "server"))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PaymentStatus {
	Success,
	Pending,
	Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CardFundingType {
	Debit,
	Credit,
	Prepaid,
	Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentMethod {
	pub id: String,
	pub customer: String,
	pub card: Option<Card>,
	//TODO: Add other payment methods
	pub created: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
	pub id: Uuid,
	pub month: i32,
	pub amount: u64,
	pub payment_intent_id: Option<String>,
	pub date: DateTime<Utc>,
	pub workspace_id: Uuid,
	pub transaction_type: TransactionType,
	pub payment_status: PaymentStatus,
	pub description: Option<String>,
}

#[cfg(feature = "server")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[serde(rename_all = "camelCase")]
#[sqlx(type_name = "TRANSACTION_TYPE", rename_all = "lowercase")]
pub enum TransactionType {
	Bill,
	Credits,
	Payment,
}

#[cfg(not(feature = "server"))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TransactionType {
	Bill,
	Credits,
	Payment,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceBillBreakdown {
	pub year: u32,
	pub month: u32,
	pub total_charge: u64,
	pub deployment_charge: u64,
	pub deployment_usage: Vec<DeploymentUsage>,
	pub volume_charge: u64,
	pub volume_usage: Vec<VolumeUsage>,
	pub database_charge: u64,
	pub database_usage: Vec<DatabaseUsage>,
	pub static_site_charge: u64,
	pub static_site_usage: Vec<StaticSiteUsage>,
	pub domain_charge: u64,
	pub domain_usage: Vec<DomainUsage>,
	pub managed_url_charge: u64,
	pub managed_url_usage: Vec<ManagedUrlUsage>,
	pub secret_charge: u64,
	pub secret_usage: Vec<SecretUsage>,
	pub docker_repository_charge: u64,
	pub docker_repository_usage: Vec<DockerRepositoryUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentUsage {
	pub name: String,
	pub deployment_id: Uuid,
	pub deployment_bill: Vec<DeploymentBill>,
	pub is_deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentBill {
	pub start_time: DateTime<Utc>,
	pub stop_time: Option<DateTime<Utc>>,
	pub machine_type: DeploymentMachineType,
	pub num_instances: u32,
	pub hours: u64,
	pub amount: u64,
	pub monthly_charge: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VolumeUsage {
	pub start_time: DateTime<Utc>,
	pub stop_time: Option<DateTime<Utc>>,
	pub storage: u64,
	pub number_of_volume: u32,
	pub hours: u64,
	pub amount: u64,
	pub monthly_charge: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseUsage {
	pub start_time: DateTime<Utc>,
	pub deletion_time: Option<DateTime<Utc>>,
	pub database_id: Uuid,
	pub name: String,
	pub hours: u64,
	pub amount: u64,
	pub is_deleted: bool,
	pub monthly_charge: u64,
	pub plan: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StaticSiteUsage {
	pub start_time: DateTime<Utc>,
	pub stop_time: Option<DateTime<Utc>>,
	pub plan: StaticSitePlan,
	pub hours: u64,
	pub amount: u64,
	pub monthly_charge: u64,
}

#[derive(
	Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Ord, PartialOrd,
)]
pub enum StaticSitePlan {
	Free,
	Pro,
	Unlimited,
}

impl fmt::Display for StaticSitePlan {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			StaticSitePlan::Free => write!(f, "Free"),
			StaticSitePlan::Pro => write!(f, "Pro"),
			StaticSitePlan::Unlimited => write!(f, "Unlimited"),
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DomainUsage {
	pub start_time: DateTime<Utc>,
	pub stop_time: Option<DateTime<Utc>>,
	pub plan: DomainPlan,
	pub hours: u64,
	pub amount: u64,
	pub monthly_charge: u64,
}

#[derive(
	Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Ord, PartialOrd,
)]
pub enum DomainPlan {
	Free,
	Unlimited,
}

impl fmt::Display for DomainPlan {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			DomainPlan::Free => write!(f, "free"),
			DomainPlan::Unlimited => write!(f, "unlimited"),
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ManagedUrlUsage {
	pub start_time: DateTime<Utc>,
	pub stop_time: Option<DateTime<Utc>>,
	pub plan: String,
	pub hours: u64,
	pub amount: u64,
	pub monthly_charge: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SecretUsage {
	pub start_time: DateTime<Utc>,
	pub stop_time: Option<DateTime<Utc>>,
	pub plan: String,
	pub hours: u64,
	pub amount: u64,
	pub monthly_charge: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DockerRepositoryUsage {
	pub start_time: DateTime<Utc>,
	pub stop_time: Option<DateTime<Utc>>,
	pub plan: String,
	pub hours: u64,
	pub amount: u64,
	pub monthly_charge: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TotalAmount {
	CreditsLeft(u64),
	NeedToPay(u64),
}

impl Add for TotalAmount {
	type Output = Self;

	fn add(self, other: Self) -> Self {
		fn abs_value(value: TotalAmount) -> i64 {
			match value {
				TotalAmount::CreditsLeft(value) => -(value as i64),
				TotalAmount::NeedToPay(value) => value as i64,
			}
		}

		let self_value = abs_value(self);
		let other_value = abs_value(other);

		let total_value = self_value + other_value;

		if total_value.is_positive() {
			TotalAmount::NeedToPay(total_value as u64)
		} else {
			TotalAmount::CreditsLeft((-total_value) as u64)
		}
	}
}

impl From<i64> for TotalAmount {
	fn from(value: i64) -> Self {
		if value.is_positive() {
			TotalAmount::NeedToPay(value as u64)
		} else {
			TotalAmount::CreditsLeft((-value) as u64)
		}
	}
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{
		DatabaseUsage,
		DeploymentBill,
		DeploymentUsage,
		DockerRepositoryUsage,
		DomainPlan,
		DomainUsage,
		ManagedUrlUsage,
		SecretUsage,
		StaticSitePlan,
		StaticSiteUsage,
		VolumeUsage,
		WorkspaceBillBreakdown,
	};
	use crate::{
		models::workspace::infrastructure::list_all_deployment_machine_type::DeploymentMachineType,
		utils::{DateTime, Uuid},
	};

	#[test]
	fn assert_workspace_bill_breakdown() {
		assert_tokens(
			&WorkspaceBillBreakdown {
				year: 2022,
				month: 5,
				total_charge: 22400,
				deployment_charge: 3200,
				deployment_usage: vec![DeploymentUsage {
					name: "test-deplo".to_string(),
					deployment_id: Uuid::parse_str(
						"d5727fb4-9e6b-43df-8a46-0c698340fffb",
					)
					.unwrap(),
					deployment_bill: vec![DeploymentBill {
						start_time: DateTime::default(),
						stop_time: Some(DateTime::default()),
						machine_type: DeploymentMachineType {
							id: Uuid::parse_str(
								"d5727fb4-9e6b-43df-8a46-0c698340fffb",
							)
							.unwrap(),
							cpu_count: 1,
							memory_count: 2,
						},
						amount: 3200,
						num_instances: 2,
						hours: 720,
						monthly_charge: 3200,
					}],
					is_deleted: false,
				}],
				volume_charge: 3200,
				volume_usage: vec![VolumeUsage {
					start_time: DateTime::default(),
					stop_time: Some(DateTime::default()),
					storage: 500,
					number_of_volume: 5,
					hours: 720,
					amount: 3200,
					monthly_charge: 3200,
				}],
				database_charge: 3200,
				database_usage: vec![DatabaseUsage {
					start_time: DateTime::default(),
					deletion_time: Some(DateTime::default()),
					database_id: Uuid::parse_str(
						"d5727fb4-9e6b-43df-8a46-0c698340fffb",
					)
					.unwrap(),
					name: "mydb".to_string(),
					hours: 720,
					amount: 3200,
					is_deleted: false,
					monthly_charge: 3200,
					plan: "micro".to_string(),
				}],
				static_site_charge: 3200,
				static_site_usage: vec![StaticSiteUsage {
					start_time: DateTime::default(),
					stop_time: Some(DateTime::default()),
					plan: StaticSitePlan::Free,
					hours: 720,
					amount: 3200,
					monthly_charge: 3200,
				}],
				domain_charge: 3200,
				domain_usage: vec![DomainUsage {
					start_time: DateTime::default(),
					stop_time: Some(DateTime::default()),
					plan: DomainPlan::Free,
					hours: 720,
					amount: 3200,
					monthly_charge: 3200,
				}],
				managed_url_charge: 3200,
				managed_url_usage: vec![ManagedUrlUsage {
					start_time: DateTime::default(),
					stop_time: Some(DateTime::default()),
					plan: "overused".to_string(),
					hours: 720,
					amount: 3200,
					monthly_charge: 3200,
				}],
				secret_charge: 3200,
				secret_usage: vec![SecretUsage {
					start_time: DateTime::default(),
					stop_time: Some(DateTime::default()),
					plan: "overused".to_string(),
					hours: 720,
					amount: 3200,
					monthly_charge: 3200,
				}],
				docker_repository_charge: 3200,
				docker_repository_usage: vec![DockerRepositoryUsage {
					start_time: DateTime::default(),
					stop_time: Some(DateTime::default()),
					plan: "overused".to_string(),
					hours: 720,
					amount: 3200,
					monthly_charge: 3200,
				}],
			},
			&[
				Token::Struct {
					name: "WorkspaceBillBreakdown",
					len: 19,
				},
				Token::Str("year"),
				Token::U32(2022),
				Token::Str("month"),
				Token::U32(5),
				Token::Str("totalCharge"),
				Token::U64(22400),
				Token::Str("deploymentCharge"),
				Token::U64(3200),
				Token::Str("deploymentUsage"),
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "DeploymentUsage",
					len: 4,
				},
				Token::Str("name"),
				Token::Str("test-deplo"),
				Token::Str("deploymentId"),
				Token::Str("d5727fb49e6b43df8a460c698340fffb"),
				Token::Str("deploymentBill"),
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "DeploymentBill",
					len: 7,
				},
				Token::Str("startTime"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("stopTime"),
				Token::Some,
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("machineType"),
				Token::Struct {
					name: "DeploymentMachineType",
					len: 3,
				},
				Token::Str("id"),
				Token::Str("d5727fb49e6b43df8a460c698340fffb"),
				Token::Str("cpuCount"),
				Token::I16(1),
				Token::Str("memoryCount"),
				Token::I32(2),
				Token::StructEnd,
				Token::Str("numInstances"),
				Token::U32(2),
				Token::Str("hours"),
				Token::U64(720),
				Token::Str("amount"),
				Token::U64(3200),
				Token::Str("monthlyCharge"),
				Token::U64(3200),
				Token::StructEnd,
				Token::SeqEnd,
				Token::Str("isDeleted"),
				Token::Bool(false),
				Token::StructEnd,
				Token::SeqEnd,
				Token::Str("volumeCharge"),
				Token::U64(3200),
				Token::Str("volumeUsage"),
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "VolumeUsage",
					len: 7,
				},
				Token::Str("startTime"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("stopTime"),
				Token::Some,
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("storage"),
				Token::U64(500),
				Token::Str("numberOfVolume"),
				Token::U32(5),
				Token::Str("hours"),
				Token::U64(720),
				Token::Str("amount"),
				Token::U64(3200),
				Token::Str("monthlyCharge"),
				Token::U64(3200),
				Token::StructEnd,
				Token::SeqEnd,
				Token::Str("databaseCharge"),
				Token::U64(3200),
				Token::Str("databaseUsage"),
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "DatabaseUsage",
					len: 9,
				},
				Token::Str("startTime"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("deletionTime"),
				Token::Some,
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("databaseId"),
				Token::Str("d5727fb49e6b43df8a460c698340fffb"),
				Token::Str("name"),
				Token::Str("mydb"),
				Token::Str("hours"),
				Token::U64(720),
				Token::Str("amount"),
				Token::U64(3200),
				Token::Str("isDeleted"),
				Token::Bool(false),
				Token::Str("monthlyCharge"),
				Token::U64(3200),
				Token::Str("plan"),
				Token::Str("micro"),
				Token::StructEnd,
				Token::SeqEnd,
				Token::Str("staticSiteCharge"),
				Token::U64(3200),
				Token::Str("staticSiteUsage"),
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "StaticSiteUsage",
					len: 6,
				},
				Token::Str("startTime"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("stopTime"),
				Token::Some,
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("plan"),
				Token::UnitVariant {
					name: "StaticSitePlan",
					variant: "Free",
				},
				Token::Str("hours"),
				Token::U64(720),
				Token::Str("amount"),
				Token::U64(3200),
				Token::Str("monthlyCharge"),
				Token::U64(3200),
				Token::StructEnd,
				Token::SeqEnd,
				Token::Str("domainCharge"),
				Token::U64(3200),
				Token::Str("domainUsage"),
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "DomainUsage",
					len: 6,
				},
				Token::Str("startTime"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("stopTime"),
				Token::Some,
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("plan"),
				Token::UnitVariant {
					name: "DomainPlan",
					variant: "Free",
				},
				Token::Str("hours"),
				Token::U64(720),
				Token::Str("amount"),
				Token::U64(3200),
				Token::Str("monthlyCharge"),
				Token::U64(3200),
				Token::StructEnd,
				Token::SeqEnd,
				Token::Str("managedUrlCharge"),
				Token::U64(3200),
				Token::Str("managedUrlUsage"),
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "ManagedUrlUsage",
					len: 6,
				},
				Token::Str("startTime"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("stopTime"),
				Token::Some,
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("plan"),
				Token::Str("overused"),
				Token::Str("hours"),
				Token::U64(720),
				Token::Str("amount"),
				Token::U64(3200),
				Token::Str("monthlyCharge"),
				Token::U64(3200),
				Token::StructEnd,
				Token::SeqEnd,
				Token::Str("secretCharge"),
				Token::U64(3200),
				Token::Str("secretUsage"),
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "SecretUsage",
					len: 6,
				},
				Token::Str("startTime"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("stopTime"),
				Token::Some,
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("plan"),
				Token::Str("overused"),
				Token::Str("hours"),
				Token::U64(720),
				Token::Str("amount"),
				Token::U64(3200),
				Token::Str("monthlyCharge"),
				Token::U64(3200),
				Token::StructEnd,
				Token::SeqEnd,
				Token::Str("dockerRepositoryCharge"),
				Token::U64(3200),
				Token::Str("dockerRepositoryUsage"),
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "DockerRepositoryUsage",
					len: 6,
				},
				Token::Str("startTime"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("stopTime"),
				Token::Some,
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("plan"),
				Token::Str("overused"),
				Token::Str("hours"),
				Token::U64(720),
				Token::Str("amount"),
				Token::U64(3200),
				Token::Str("monthlyCharge"),
				Token::U64(3200),
				Token::StructEnd,
				Token::SeqEnd,
				Token::StructEnd,
			],
		);
	}
}
