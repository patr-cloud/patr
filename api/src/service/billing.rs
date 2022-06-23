use std::collections::HashMap;

use api_models::{
	models::workspace::billing::PaymentMethod,
	utils::{DateTime, True, Uuid},
};
use chrono::Utc;
use eve_rs::AsError;
use reqwest::Client;
use serde_json::json;

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
) -> Result<String, Error> {
	let client = Client::new();

	let password: Option<String> = None;

	let payment_intent_object = client
		.post("https://api.stripe.com/v1/payment_intents")
		.basic_auth(&config.stripe.secret_key, password)
		.form(&PaymentIntent {
			amount: (credits * 10).into(),
			currency: "usd".to_string(),
			confirm: True,
			off_session: True,
			description: "Patr charge: Additional credits".to_string(),
			customer: db::get_workspace_info(connection, workspace_id)
				.await?
				.status(500)?
				.stripe_customer_id,
			payment_method: None,
			payment_method_types: "card".to_string(),
			setup_future_usage: "off_session".to_string(),
		})
		.send()
		.await?
		.error_for_status()?
		.json::<PaymentIntentObject>()
		.await?;

	let metadata = json!({
		"payment_intent_id": payment_intent_object.id,
		"status": payment_intent_object.status
	});

	db::add_credits_to_workspace(
		connection,
		workspace_id,
		credits.into(),
		&metadata,
		Utc::now().into(),
	)
	.await?;

	Ok(payment_intent_object.client_secret)
}

pub async fn confirm_payment_method(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	payment_intent_id: &str,
	config: &Settings,
) -> Result<bool, Error> {
	let client = Client::new();
	let password: Option<String> = None;

	let payment_info =
		db::get_credit_info(connection, workspace_id, payment_intent_id)
			.await?
			.status(500)?;

	let payment_id = payment_info
		.clone()
		.metadata
		.get("payment_intent_id")
		.and_then(|value| value.as_str())
		.and_then(|c| c.parse::<String>().ok())
		.status(500)?;

	if payment_intent_id != payment_id {
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

	let metadata = json!({
		"payment_intent_id": payment_intent_object.id.clone(),
		"status": payment_intent_object.status
	});

	db::update_workspace_credit_metadata(
		connection,
		workspace_id,
		&metadata,
		&payment_intent_object.id,
	)
	.await?;

	if payment_intent_object.status != Some(PaymentMethodStatus::Succeeded) &&
		payment_intent_object.amount == payment_info.credits as f64
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
		let hours = (deployment_usage
			.stop_time
			.map(chrono::DateTime::from)
			.unwrap_or_else(|| *till_date) -
			chrono::DateTime::from(deployment_usage.start_time))
		.num_hours();

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
					let price = (hours - remaining_free_hours).max(0) as f64 *
						monthly_price;
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
		let hours = (database_usage
			.deletion_time
			.map(chrono::DateTime::from)
			.unwrap_or_else(|| *till_date) -
			chrono::DateTime::from(database_usage.start_time))
		.num_hours();

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
				amount: if hours > 720 {
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
		let hours = (static_sites_usage
			.stop_time
			.map(chrono::DateTime::from)
			.unwrap_or_else(|| *till_date) -
			chrono::DateTime::from(static_sites_usage.start_time))
		.num_hours();
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
		bill.amount = if bill.hours > 720 {
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
		let hours = (managed_url_usage
			.stop_time
			.map(chrono::DateTime::from)
			.unwrap_or_else(|| *till_date) -
			chrono::DateTime::from(managed_url_usage.start_time))
		.num_hours();
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
		bill.amount = if bill.hours > 720 {
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
		let hours = (docker_repository_usage
			.stop_time
			.map(chrono::DateTime::from)
			.unwrap_or_else(|| *till_date) -
			chrono::DateTime::from(docker_repository_usage.start_time))
		.num_hours();
		let monthly_price = if docker_repository_usage.storage <= 10 {
			0f64
		} else if docker_repository_usage.storage > 10 &&
			docker_repository_usage.storage <= 100
		{
			10f64
		} else {
			(docker_repository_usage.storage as f64 * 0.1f64).ceil()
		};

		docker_repository_bill.push(DockerRepositoryBill {
			storage: docker_repository_usage.storage as u64,
			hours: hours as u64,
			amount: if hours > 720 {
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
		let hours = (domains_usage
			.stop_time
			.map(chrono::DateTime::from)
			.unwrap_or_else(|| *till_date) -
			chrono::DateTime::from(domains_usage.start_time))
		.num_hours();
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
		bill.amount = if hours > 720 {
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
		let hours = (secrets_usage
			.stop_time
			.map(chrono::DateTime::from)
			.unwrap_or_else(|| *till_date) -
			chrono::DateTime::from(secrets_usage.start_time))
		.num_hours();
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
		bill.amount = if bill.hours > 720 {
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
	let stripe_customer_id = db::get_workspace_info(connection, workspace_id)
		.await?
		.status(500)?
		.stripe_customer_id;
	let password: Option<String> = None;
	client
		.post("https://api.stripe.com/v1/setup_intents")
		.basic_auth(&config.stripe.secret_key, password)
		.query(&[
			("customer", stripe_customer_id.as_str()),
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
