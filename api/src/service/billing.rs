use std::{
	cmp::{max, min},
	collections::HashMap,
};

use api_models::{
	models::workspace::billing::{
		PaymentMethod,
		PaymentStatus,
		TransactionType,
	},
	utils::{DateTime, True, Uuid},
};
use chrono::{Datelike, Utc};
use eve_rs::AsError;
use reqwest::Client;

use crate::{
	db::{self, DomainPlan, ManagedDatabasePlan, StaticSitePlan},
	error,
	models::{
		billing::{
			DatabaseBill,
			DeploymentBill,
			DeploymentBillItem,
			DockerRepositoryBill,
			DomainBill,
			ManagedUrlBill,
			PaymentIntent,
			PaymentIntentObject,
			PaymentMethodStatus,
			SecretsBill,
			StaticSiteBill,
		},
		deployment,
	},
	utils::{settings::Settings, Error},
	Database,
};

pub async fn add_credits_to_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	credits: u32,
	config: &Settings,
) -> Result<(), Error> {
	let client = Client::new();

	let password: Option<String> = None;

	let default_payment_method_id =
		db::get_workspace_info(connection, workspace_id)
			.await?
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?
			.default_payment_method_id;

	if default_payment_method_id.is_none() {
		return Error::as_result()
			.status(402)
			.body(error!(PAYMENT_METHOD_REQUIRED).to_string())?;
	}

	let address_id = db::get_workspace_info(connection, workspace_id)
		.await?
		.status(500)?
		.address_id
		.status(400)
		.body(error!(ADDRESS_REQUIRED).to_string())?;

	const EARLY_ADOPTER_50: &str = "Patr Early Adopter 50";
	const EARLY_ADOPTER_125: &str = "Patr Early Adopter 125";
	const NORMAL_CREDITS: &str = "Patr charge: Additional credits";

	let (dollars, description) = if credits == 50 {
		let transaction_exists =
			db::get_transaction_by_description_in_workspace(
				connection,
				workspace_id,
				EARLY_ADOPTER_50,
			)
			.await?;
		if transaction_exists.is_some() {
			// This transaction has already been purchased
			(50, NORMAL_CREDITS)
		} else {
			(10, EARLY_ADOPTER_50)
		}
	} else if credits == 125 {
		let transaction_exists =
			db::get_transaction_by_description_in_workspace(
				connection,
				workspace_id,
				EARLY_ADOPTER_125,
			)
			.await?;
		if transaction_exists.is_some() {
			// This transaction has already been purchased
			(125, NORMAL_CREDITS)
		} else {
			(25, EARLY_ADOPTER_125)
		}
	} else {
		(credits, NORMAL_CREDITS)
	};

	let (currency, amount) = if db::get_billing_address(connection, &address_id)
		.await?
		.status(500)?
		.country == *"IN"
	{
		("inr".to_string(), (dollars * 100 * 80) as u64)
	} else {
		("usd".to_string(), (dollars * 100) as u64)
	};

	let description = description.to_string();

	let payment_intent_object = client
		.post("https://api.stripe.com/v1/payment_intents")
		.basic_auth(&config.stripe.secret_key, password)
		.form(&PaymentIntent {
			amount,
			currency,
			confirm: True,
			off_session: true,
			description: description.clone(),
			customer: db::get_workspace_info(connection, workspace_id)
				.await?
				.status(500)?
				.stripe_customer_id,
			payment_method: default_payment_method_id,
			payment_method_types: "card".to_string(),
			setup_future_usage: None,
		})
		.send()
		.await?
		.error_for_status()?
		.json::<PaymentIntentObject>()
		.await?;

	let id = db::generate_new_transaction_id(connection).await?;

	let date = Utc::now();

	db::create_transaction(
		connection,
		workspace_id,
		&id,
		date.month() as i32,
		credits.into(),
		Some(&payment_intent_object.id),
		&DateTime::from(date),
		&TransactionType::Credits,
		&PaymentStatus::Success,
		Some(&description),
	)
	.await?;

	Ok(())
}

pub async fn confirm_payment_method(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	payment_intent_id: &str,
	config: &Settings,
) -> Result<bool, Error> {
	let client = Client::new();
	let password: Option<String> = None;

	let transaction = db::get_transaction_by_payment_intent_id_in_workspace(
		connection,
		workspace_id,
		payment_intent_id,
	)
	.await?
	.status(500)?;

	if Some(payment_intent_id) != transaction.payment_intent_id.as_deref() {
		return Error::as_result()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string())?;
	}

	let payment_intent_object = client
		.get(
			format!(
				"https://api.stripe.com/v1/payment_intents/{}",
				payment_intent_id
			)
			.as_str(),
		)
		.basic_auth(&config.stripe.secret_key, password)
		.send()
		.await?
		.json::<PaymentIntentObject>()
		.await?;

	if payment_intent_object.status != Some(PaymentMethodStatus::Succeeded) &&
		payment_intent_object.amount == Some(transaction.amount)
	{
		return Ok(false);
	}

	Ok(true)
}

pub async fn calculate_deployment_bill_for_workspace_till(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	month_start_date: &chrono::DateTime<Utc>,
	till_date: &chrono::DateTime<Utc>,
) -> Result<HashMap<Uuid, DeploymentBill>, Error> {
	let deployment_usages = db::get_all_deployment_usage(
		&mut *connection,
		workspace_id,
		&DateTime::from(*month_start_date),
	)
	.await?;

	let mut deployment_usage_bill = HashMap::new();
	let mut remaining_free_hours = 720;

	for deployment_usage in deployment_usages {
		let stop_time = deployment_usage
			.stop_time
			.map(chrono::DateTime::from)
			.unwrap_or_else(|| *till_date);
		let start_time = max(deployment_usage.start_time, *month_start_date);
		let hours = min(
			720,
			((stop_time - start_time).num_seconds() as f64 / 3600f64).ceil()
				as i64,
		);

		let (cpu_count, memory_count) = deployment::MACHINE_TYPES
			.get()
			.unwrap()
			.get(&deployment_usage.machine_type)
			.unwrap_or(&(1, 2));

		let monthly_price = match (cpu_count, memory_count) {
			(1, 2) => 5f64,
			(1, 4) => 10f64,
			(1, 8) => 20f64,
			(2, 8) => 40f64,
			(4, 32) => 80f64,
			_ => 0f64,
		};
		deployment_usage_bill
			.entry(deployment_usage.deployment_id.clone())
			.or_insert(DeploymentBill {
				deployment_id: deployment_usage.deployment_id.clone(),
				deployment_name: db::get_deployment_by_id_including_deleted(
					connection,
					&deployment_usage.deployment_id,
				)
				.await?
				.map(|deployment| {
					deployment
						.name
						.replace("patr-deleted: ", "")
						.replace(format!("-{}", deployment.id).as_str(), "")
				})
				.as_deref()
				.unwrap_or("unknown")
				.to_string(),
				bill_items: vec![],
			})
			.bill_items
			.push(DeploymentBillItem {
				machine_type: (*cpu_count as u16, *memory_count as u32),
				num_instances: deployment_usage.num_instance as u32,
				hours: hours as u64,
				amount: if (*cpu_count, *memory_count) == (1, 2) &&
					deployment_usage.num_instance == 1
				{
					// Free eligible
					let price = (((hours - remaining_free_hours).max(0)
						as f64) / 720f64) * monthly_price;
					remaining_free_hours =
						(remaining_free_hours - hours).max(0);
					price
				} else if hours >= 720 {
					monthly_price
				} else {
					((hours as f64) / 720f64) * monthly_price
				},
			});
	}

	Ok(deployment_usage_bill)
}

pub async fn calculate_database_bill_for_workspace_till(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	month_start_date: &chrono::DateTime<Utc>,
	till_date: &chrono::DateTime<Utc>,
) -> Result<HashMap<Uuid, DatabaseBill>, Error> {
	let database_usages = db::get_all_database_usage(
		&mut *connection,
		workspace_id,
		&DateTime::from(*month_start_date),
	)
	.await?;

	let mut database_usage_bill = HashMap::new();

	for database_usage in database_usages {
		let stop_time = database_usage
			.deletion_time
			.map(chrono::DateTime::from)
			.unwrap_or_else(|| *till_date);
		let start_time = max(database_usage.start_time, *month_start_date);
		let hours = min(
			720,
			((stop_time - start_time).num_seconds() as f64 / 3600f64).ceil()
				as i64,
		);

		let monthly_price = match database_usage.db_plan {
			ManagedDatabasePlan::Nano => 15f64,
			ManagedDatabasePlan::Micro => 30f64,
			ManagedDatabasePlan::Small => 45f64,
			ManagedDatabasePlan::Medium => 60f64,
			ManagedDatabasePlan::Large => 120f64,
			ManagedDatabasePlan::Xlarge => 240f64,
			ManagedDatabasePlan::Xxlarge => 480f64,
			ManagedDatabasePlan::Mammoth => 960f64,
		};

		database_usage_bill
			.entry(database_usage.database_id.clone())
			.or_insert(DatabaseBill {
				database_id: database_usage.database_id.clone(),
				database_name:
					db::get_managed_database_by_id_including_deleted(
						connection,
						&database_usage.database_id,
					)
					.await?
					.map(|database| {
						database
							.name
							.replace("patr-deleted: ", "")
							.replace(format!("-{}", database.id).as_str(), "")
					})
					.as_deref()
					.unwrap_or("unknown")
					.to_string(),
				hours: hours as u64,
				amount: if hours >= 720 {
					monthly_price
				} else {
					(hours as f64 / 720f64) * monthly_price
				},
			});
	}

	Ok(database_usage_bill)
}

pub async fn calculate_static_sites_bill_for_workspace_till(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	month_start_date: &chrono::DateTime<Utc>,
	till_date: &chrono::DateTime<Utc>,
) -> Result<HashMap<StaticSitePlan, StaticSiteBill>, Error> {
	let static_sites_usages = db::get_all_static_site_usages(
		&mut *connection,
		workspace_id,
		&DateTime::from(*month_start_date),
	)
	.await?;

	let mut static_sites_bill = HashMap::new();
	for static_sites_usage in static_sites_usages {
		let stop_time = static_sites_usage
			.stop_time
			.map(chrono::DateTime::from)
			.unwrap_or_else(|| *till_date);
		let start_time = max(static_sites_usage.start_time, *month_start_date);
		let hours = min(
			720,
			((stop_time - start_time).num_seconds() as f64 / 3600f64).ceil()
				as i64,
		);

		let monthly_price = match static_sites_usage.static_site_plan {
			StaticSitePlan::Free => 0f64,
			StaticSitePlan::Pro => 5f64,
			StaticSitePlan::Unlimited => 10f64,
		};
		let bill = static_sites_bill
			.entry(static_sites_usage.static_site_plan)
			.or_insert(StaticSiteBill {
				hours: 0,
				amount: 0f64,
			});
		bill.hours += hours as u64;
		bill.amount = if bill.hours >= 720 {
			monthly_price
		} else {
			(bill.hours as f64 / 720f64) * monthly_price
		};
	}

	Ok(static_sites_bill)
}

pub async fn calculate_managed_urls_bill_for_workspace_till(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	month_start_date: &chrono::DateTime<Utc>,
	till_date: &chrono::DateTime<Utc>,
) -> Result<HashMap<u64, ManagedUrlBill>, Error> {
	let managed_url_usages = db::get_all_managed_url_usages(
		&mut *connection,
		workspace_id,
		&DateTime::from(*month_start_date),
	)
	.await?;

	let mut managed_url_bill = HashMap::new();
	for managed_url_usage in managed_url_usages {
		let stop_time = managed_url_usage
			.stop_time
			.map(chrono::DateTime::from)
			.unwrap_or_else(|| *till_date);
		let start_time = max(managed_url_usage.start_time, *month_start_date);
		let hours = min(
			720,
			((stop_time - start_time).num_seconds() as f64 / 3600f64).ceil()
				as i64,
		);

		let monthly_price = if managed_url_usage.url_count <= 10 {
			0f64
		} else {
			(managed_url_usage.url_count as f64 / 100f64).ceil() * 10f64
		};

		let bill = managed_url_bill
			.entry(
				if managed_url_usage.url_count <= 10 {
					0
				} else {
					(managed_url_usage.url_count as f64 / 100f64).ceil() as u64
				},
			)
			.or_insert(ManagedUrlBill {
				url_count: 0,
				hours: 0,
				amount: 0f64,
			});

		bill.url_count = managed_url_usage.url_count as u64;
		bill.hours += hours as u64;
		bill.amount = if bill.hours >= 720 {
			monthly_price
		} else {
			(bill.hours as f64 / 720f64) * monthly_price
		};
	}

	Ok(managed_url_bill)
}

pub async fn calculate_docker_repository_bill_for_workspace_till(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	month_start_date: &chrono::DateTime<Utc>,
	till_date: &chrono::DateTime<Utc>,
) -> Result<Vec<DockerRepositoryBill>, Error> {
	let docker_repository_usages = db::get_all_docker_repository_usages(
		&mut *connection,
		workspace_id,
		&DateTime::from(*month_start_date),
	)
	.await?;

	let mut docker_repository_bill = Vec::new();

	for docker_repository_usage in docker_repository_usages {
		let stop_time = docker_repository_usage
			.stop_time
			.map(chrono::DateTime::from)
			.unwrap_or_else(|| *till_date);
		let start_time =
			max(docker_repository_usage.start_time, *month_start_date);
		let hours = min(
			720,
			((stop_time - start_time).num_seconds() as f64 / 3600f64).ceil()
				as i64,
		);

		let storage_in_gb = (docker_repository_usage.storage as f64 /
			(1024 * 1024 * 1024) as f64)
			.ceil() as i64;
		let monthly_price = if storage_in_gb <= 10 {
			0f64
		} else if storage_in_gb > 10 && storage_in_gb <= 100 {
			10f64
		} else {
			(storage_in_gb as f64 * 0.1f64).ceil()
		};

		docker_repository_bill.push(DockerRepositoryBill {
			storage: docker_repository_usage.storage as u64,
			hours: hours as u64,
			amount: if hours >= 720 {
				monthly_price
			} else {
				(hours as f64 / 720f64) * monthly_price
			},
		});
	}

	Ok(docker_repository_bill)
}

pub async fn calculate_domains_bill_for_workspace_till(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	month_start_date: &chrono::DateTime<Utc>,
	till_date: &chrono::DateTime<Utc>,
) -> Result<HashMap<DomainPlan, DomainBill>, Error> {
	let domains_usages = db::get_all_domains_usages(
		&mut *connection,
		workspace_id,
		&DateTime::from(*month_start_date),
	)
	.await?;

	let mut domains_bill = HashMap::new();
	for domains_usage in domains_usages {
		let stop_time = domains_usage
			.stop_time
			.map(chrono::DateTime::from)
			.unwrap_or_else(|| *till_date);
		let start_time = max(domains_usage.start_time, *month_start_date);
		let hours = min(
			720,
			((stop_time - start_time).num_seconds() as f64 / 3600f64).ceil()
				as i64,
		);

		let monthly_price = match domains_usage.domain_plan {
			DomainPlan::Free => 0f64,
			DomainPlan::Unlimited => 10f64,
		};
		let bill = domains_bill.entry(domains_usage.domain_plan).or_insert(
			DomainBill {
				hours: 0,
				amount: 0f64,
			},
		);

		bill.hours = hours as u64;
		bill.amount = if hours >= 720 {
			monthly_price
		} else {
			hours as f64 * (monthly_price / 720f64)
		};
	}

	Ok(domains_bill)
}

pub async fn calculate_secrets_bill_for_workspace_till(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	month_start_date: &chrono::DateTime<Utc>,
	till_date: &chrono::DateTime<Utc>,
) -> Result<HashMap<u64, SecretsBill>, Error> {
	let secrets_usages = db::get_all_secrets_usages(
		&mut *connection,
		workspace_id,
		&DateTime::from(*month_start_date),
	)
	.await?;

	let mut secrets_bill = HashMap::new();
	for secrets_usage in secrets_usages {
		let stop_time = secrets_usage
			.stop_time
			.map(chrono::DateTime::from)
			.unwrap_or_else(|| *till_date);
		let start_time = max(secrets_usage.start_time, *month_start_date);
		let hours = min(
			720,
			((stop_time - start_time).num_seconds() as f64 / 3600f64).ceil()
				as i64,
		);

		let monthly_price = if secrets_usage.secret_count <= 3 {
			0f64
		} else {
			(secrets_usage.secret_count as f64 / 100f64).ceil() * 10f64
		};

		let bill = secrets_bill
			.entry(
				if secrets_usage.secret_count <= 3 {
					0
				} else {
					(secrets_usage.secret_count as f64 / 100f64).ceil() as u64
				},
			)
			.or_insert(SecretsBill {
				secrets_count: 0,
				hours: 0,
				amount: 0f64,
			});

		bill.secrets_count = secrets_usage.secret_count as u64;
		bill.hours += hours as u64;
		bill.amount = if bill.hours >= 720 {
			monthly_price
		} else {
			(bill.hours as f64 / 720f64) * monthly_price
		};
	}

	Ok(secrets_bill)
}

pub async fn add_card_details(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	config: &Settings,
) -> Result<PaymentIntentObject, Error> {
	let client = Client::new();
	let workspace = db::get_workspace_info(connection, workspace_id)
		.await?
		.status(500)?;

	if workspace.address_id.is_none() {
		return Error::as_result()
			.status(400)
			.body(error!(ADDRESS_REQUIRED).to_string())?;
	}

	let password: Option<String> = None;
	client
		.post("https://api.stripe.com/v1/setup_intents")
		.basic_auth(&config.stripe.secret_key, password)
		.query(&[
			("customer", workspace.stripe_customer_id.as_str()),
			// for now only accepting cards, other payment methods will be
			// accepted at later point of time
			("payment_method_types[]", "card"),
		])
		.send()
		.await?
		.json::<PaymentIntentObject>()
		.await
		.map_err(|e| e.into())
}

pub async fn get_card_details(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	config: &Settings,
) -> Result<Vec<PaymentMethod>, Error> {
	let payment_source_list =
		db::get_payment_methods_for_workspace(connection, workspace_id).await?;

	let mut cards = Vec::new();
	let client = Client::new();
	for payment_source in payment_source_list {
		let url = format!(
			"https://api.stripe.com/v1/payment_methods/{}",
			payment_source.payment_method_id
		);
		let password: Option<String> = None;
		let card_details = client
			.get(&url)
			.basic_auth(&config.stripe.secret_key, password)
			.send()
			.await?
			.json::<PaymentMethod>()
			.await?;
		cards.push(card_details);
	}
	Ok(cards)
}

pub async fn delete_payment_method(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	payment_method_id: &str,
	config: &Settings,
) -> Result<(), Error> {
	// TODO: check for active resources above the free plan
	// if there are any check if this is only the payment method the user has
	// if that is the case throw an error

	let payment_methods =
		db::get_payment_methods_for_workspace(connection, workspace_id).await?;
	if payment_methods.len() == 1 {
		return Error::as_result()
			.status(400)
			.body(error!(CANNOT_DELETE_PAYMENT_METHOD).to_string())?;
	}

	// check if the payment method is primary
	let primary_payment_method =
		db::get_workspace_info(connection, workspace_id)
			.await?
			.status(500)?
			.default_payment_method_id
			.status(500)?;
	if payment_method_id == primary_payment_method {
		return Error::as_result()
			.status(400)
			.body(error!(CHANGE_PRIMARY_PAYMENT_METHOD).to_string())?;
	}
	db::delete_payment_method(connection, payment_method_id).await?;
	let client = Client::new();
	let password: Option<String> = None;
	let deletion_status = client
		.post(format!(
			"https://api.stripe.com/v1/payment_methods/{}/detach",
			payment_method_id
		))
		.basic_auth(&config.stripe.secret_key, password)
		.send()
		.await?
		.status();
	if !deletion_status.is_success() {
		return Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;
	}
	Ok(())
}

pub async fn calculate_total_bill_for_workspace_till(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	month_start_date: &chrono::DateTime<Utc>,
	till_date: &chrono::DateTime<Utc>,
) -> Result<f64, Error> {
	let deployment_usages = calculate_deployment_bill_for_workspace_till(
		&mut *connection,
		workspace_id,
		month_start_date,
		till_date,
	)
	.await?;

	let database_usages = calculate_database_bill_for_workspace_till(
		&mut *connection,
		workspace_id,
		month_start_date,
		till_date,
	)
	.await?;

	let static_sites_usages = calculate_static_sites_bill_for_workspace_till(
		&mut *connection,
		workspace_id,
		month_start_date,
		till_date,
	)
	.await?;

	let managed_url_usages = calculate_managed_urls_bill_for_workspace_till(
		&mut *connection,
		workspace_id,
		month_start_date,
		till_date,
	)
	.await?;

	let docker_repository_usages =
		calculate_docker_repository_bill_for_workspace_till(
			&mut *connection,
			workspace_id,
			month_start_date,
			till_date,
		)
		.await?;

	let domains_usages = calculate_domains_bill_for_workspace_till(
		&mut *connection,
		workspace_id,
		month_start_date,
		till_date,
	)
	.await?;

	let secrets_usages = calculate_secrets_bill_for_workspace_till(
		&mut *connection,
		workspace_id,
		month_start_date,
		till_date,
	)
	.await?;

	let deployment_cost = deployment_usages
		.iter()
		.map(|(_, bill)| {
			bill.bill_items.iter().map(|item| item.amount).sum::<f64>()
		})
		.sum::<f64>();
	let database_cost = database_usages
		.iter()
		.map(|(_, bill)| bill.amount)
		.sum::<f64>();
	let static_site_cost = static_sites_usages
		.iter()
		.map(|(_, bill)| bill.amount)
		.sum::<f64>();
	let managed_url_cost = managed_url_usages
		.iter()
		.map(|(_, bill)| bill.amount)
		.sum::<f64>();
	let docker_repo_cost = docker_repository_usages
		.iter()
		.map(|bill| bill.amount)
		.sum::<f64>();
	let managed_domain_cost = domains_usages
		.iter()
		.map(|(_, bill)| bill.amount)
		.sum::<f64>();
	let managed_secret_cost = secrets_usages
		.iter()
		.map(|(_, bill)| bill.amount)
		.sum::<f64>();

	let total_cost = deployment_cost +
		database_cost +
		static_site_cost +
		managed_url_cost +
		docker_repo_cost +
		managed_domain_cost +
		managed_secret_cost;

	if total_cost > 0.0 && cfg!(debug_assertions) {
		log::trace!(
			"Total bill for workspace `{}`: {}",
			workspace_id,
			serde_json::to_string(&serde_json::json!({
				"cost": total_cost,
				"deployment_usages": deployment_usages,
				"database_usages": database_usages,
				"static_sites_usages": static_sites_usages,
				"managed_url_usages": managed_url_usages,
				"docker_repository_usages": docker_repository_usages,
				"domains_usages": domains_usages,
				"secrets_usages": secrets_usages,
			}))
			.unwrap_or_default()
		);
	}

	Ok(total_cost)
}
