use std::collections::HashMap;

use api_models::{
	models::workspace::billing::PaymentMethod,
	utils::{DateTime, Uuid},
};
use chrono::Utc;
use eve_rs::AsError;
use reqwest::Client;
use serde_json::json;

use crate::{
	db::{self, DomainPlan, ManagedDatabasePlan, StaticSitePlan},
	error,
	models::{
		deployment,
		PaymentIntent,
		PaymentIntentObject,
		PaymentMethodStatus,
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
			amount: (credits * 10) as u64,
			currency: "usd".to_string(),
			description: "Patr charge: Additional credits".to_string(),
			customer: config.stripe.customer_id.clone(),
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

pub async fn calculate_total_bill_for_workspace_till(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	month_start_date: &chrono::DateTime<Utc>,
	till_date: &chrono::DateTime<Utc>,
) -> Result<u64, Error> {
	let deployment_usages = db::get_all_deployment_usage(
		&mut *connection,
		&workspace_id,
		&DateTime::from(month_start_date.clone()),
	)
	.await?;

	let database_usages = db::get_all_database_usage(
		&mut *connection,
		&workspace_id,
		&DateTime::from(month_start_date.clone()),
	)
	.await?;

	let mut deployment_bill = 0;
	for deployment_usage in deployment_usages {
		let hours = (deployment_usage
			.stop_time
			.map(|st| chrono::DateTime::from(st))
			.unwrap_or(till_date.clone()) -
			chrono::DateTime::from(deployment_usage.start_time))
		.num_hours();

		let (cpu_count, memory_count) = deployment::MACHINE_TYPES
			.get()
			.unwrap()
			.get(&deployment_usage.machine_type)
			.unwrap_or(&(1, 2));

		let monthly_price = match (cpu_count, memory_count) {
			(1, 2) => {
				if deployment_usage.num_instance == 1 {
					// TODO free deployment
					0
				} else {
					5
				}
			}
			(1, 4) => 10,
			(1, 8) => 20,
			(2, 8) => 40,
			(4, 32) => 80,
		};
		deployment_bill += if hours > 720 {
			monthly_price
		} else {
			hours as u64 * (monthly_price / 720)
		};
	}

	let mut database_bill = 0;
	for database_usage in database_usages {
		let hours = (database_usage
			.deletion_time
			.map(|st| chrono::DateTime::from(st))
			.unwrap_or(till_date.clone()) -
			chrono::DateTime::from(database_usage.start_time))
		.num_hours();
		let monthly_price = match database_usage.db_plan {
			// TODO update this
			ManagedDatabasePlan::Nano => 15,
			ManagedDatabasePlan::Micro => 30,
			ManagedDatabasePlan::Medium => 60,
			ManagedDatabasePlan::Large => 120,
			ManagedDatabasePlan::Xlarge => 240,
			ManagedDatabasePlan::Xxlarge => 480,
			ManagedDatabasePlan::Mammoth => 960,
			_ => {
				return Error::as_result()
					.status(500)
					.body(error!(SERVER_ERROR).to_string())?;
			}
		};
		database_bill += if hours > 720 {
			monthly_price
		} else {
			hours as u64 * (monthly_price / 720)
		};
	}

	let static_sites_bill = get_static_site_usage_and_bill(
		connection,
		workspace_id,
		month_start_date,
		till_date,
	)
	.await?;

	let mut managed_url_bill = get_managed_url_usage_and_bill(
		connection,
		workspace_id,
		month_start_date,
		till_date,
	)
	.await?;

	let mut docker_repository_bill = get_docker_repo_usage_and_bill(
		connection,
		workspace_id,
		month_start_date,
		till_date,
	)
	.await?;

	let domains_bill = get_domain_usage_and_bill(
		connection,
		workspace_id,
		month_start_date,
		till_date,
	)
	.await?;

	let secrets_bill = get_secret_usage_and_bill(
		connection,
		workspace_id,
		month_start_date,
		till_date,
	)
	.await?;

	Ok(deployment_bill +
		database_bill +
		static_sites_bill +
		managed_url_bill +
		docker_repository_bill +
		domains_bill +
		secrets_bill)
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
			payment_source.id
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

pub async fn get_credits_for_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<u64, Error> {
	let transactions =
		db::get_credits_for_workspace(connection, &workspace_id).await?;

	let mut credits = 0;

	for transaction in transactions {
		credits += transaction.amount;
	}

	// TODO: if credits are negative do something

	Ok(credits as u64)
}

pub async fn get_deployment_usage_and_bill(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	month_start_date: &chrono::DateTime<Utc>,
	till_date: &chrono::DateTime<Utc>,
) -> Result<HashMap<Uuid, (u64, u64)>, Error> {
	let deployments =
		db::get_deployments_for_workspace(connection, workspace_id).await?;

	let mut deployment_usage_bill = HashMap::new();

	for deployment in deployments {
		let deployment_usages = db::get_deployment_usage(
			connection,
			&deployment.id,
			&DateTime::from(month_start_date.clone()),
		)
		.await?;

		let mut deployment_bill = 0;
		let mut hours = 0;
		for deployment_usage in deployment_usages {
			hours = (deployment_usage
				.stop_time
				.map(|st| chrono::DateTime::from(st))
				.unwrap_or(till_date.clone()) -
				chrono::DateTime::from(deployment_usage.start_time))
			.num_hours();

			let (cpu_count, memory_count) = deployment::MACHINE_TYPES
				.get()
				.unwrap()
				.get(&deployment_usage.machine_type)
				.unwrap_or(&(1, 2));

			let monthly_price = match (cpu_count, memory_count) {
				(1, 2) => {
					if deployment_usage.num_instance == 1 {
						// TODO free deployment
						0
					} else {
						5
					}
				}
				(1, 4) => 10,
				(1, 8) => 20,
				(2, 8) => 40,
				(4, 32) => 80,
			};
			deployment_bill += if hours > 720 {
				monthly_price
			} else {
				hours as u64 * (monthly_price / 720)
			};
		}
		deployment_usage_bill.insert(deployment.id, (deployment_bill, hours as u64));
	}

	Ok(deployment_usage_bill)
}

pub async fn get_database_usage_and_bill(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	month_start_date: &chrono::DateTime<Utc>,
	till_date: &chrono::DateTime<Utc>,
) -> Result<HashMap<Uuid, (u64, u64)>, Error> {
	let databases =
		db::get_all_database_clusters_for_workspace(connection, workspace_id)
			.await?;

	let mut database_usage_bill = HashMap::new();
	for database in databases {
		let database_usages = db::get_database_usage(
			connection,
			&database.id,
			&DateTime::from(month_start_date.clone()),
		)
		.await?;

		let mut database_bill = 0;
		let mut hours = 0;
		for database_usage in database_usages {
			hours = (database_usage
				.deletion_time
				.map(|st| chrono::DateTime::from(st))
				.unwrap_or(till_date.clone()) -
				chrono::DateTime::from(database_usage.start_time))
			.num_hours();
			let monthly_price = match database_usage.db_plan {
				// TODO update this
				ManagedDatabasePlan::Nano => 15,
				ManagedDatabasePlan::Micro => 30,
				ManagedDatabasePlan::Medium => 60,
				ManagedDatabasePlan::Large => 120,
				ManagedDatabasePlan::Xlarge => 240,
				ManagedDatabasePlan::Xxlarge => 480,
				ManagedDatabasePlan::Mammoth => 960,
				_ => {
					return Error::as_result()
						.status(500)
						.body(error!(SERVER_ERROR).to_string())?;
				}
			};
			database_bill += if hours > 720 {
				monthly_price
			} else {
				hours as u64 * (monthly_price / 720)
			};
		}
		database_usage_bill.insert(database.id, (database_bill, hours as u64));
	}

	Ok(database_usage_bill)
}

pub async fn get_static_site_usage_and_bill(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	month_start_date: &chrono::DateTime<Utc>,
	till_date: &chrono::DateTime<Utc>,
) -> Result<u64, Error> {
	let static_sites_usages = db::get_all_static_site_usages(
		&mut *connection,
		&workspace_id,
		&DateTime::from(month_start_date.clone()),
	)
	.await?;

	let mut static_sites_bill = 0;
	for static_sites_usage in static_sites_usages {
		let hours = (static_sites_usage
			.stop_time
			.map(|st| chrono::DateTime::from(st))
			.unwrap_or(till_date.clone()) -
			chrono::DateTime::from(static_sites_usage.start_time))
		.num_hours();
		let monthly_price = match static_sites_usage.static_site_plan {
			StaticSitePlan::Free => 0,
			StaticSitePlan::Pro => 5,
			StaticSitePlan::Unlimited => 10,
		};
		static_sites_bill += if hours > 720 {
			monthly_price
		} else {
			hours as u64 * (monthly_price / 720)
		};
	}

	Ok(static_sites_bill)
}

pub async fn get_managed_url_usage_and_bill(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	month_start_date: &chrono::DateTime<Utc>,
	till_date: &chrono::DateTime<Utc>,
) -> Result<u64, Error> {
	let managed_url_usages = db::get_all_managed_url_usages(
		&mut *connection,
		&workspace_id,
		&DateTime::from(month_start_date.clone()),
	)
	.await?;

	let mut managed_url_bill = 0;
	for managed_url_usage in managed_url_usages {
		let hours = (managed_url_usage
			.stop_time
			.map(|st| chrono::DateTime::from(st))
			.unwrap_or(till_date.clone()) -
			chrono::DateTime::from(managed_url_usage.start_time))
		.num_hours();
		let monthly_price = if managed_url_usage.url_count <= 10 {
			0
		} else {
			((managed_url_usage.url_count as f64 / 100 as f64).ceil() * 10f64)
				as u64
		};
		managed_url_bill += if hours > 720 {
			monthly_price
		} else {
			hours as u64 * (monthly_price / 720)
		};
	}

	Ok(managed_url_bill)
}

pub async fn get_secret_usage_and_bill(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	month_start_date: &chrono::DateTime<Utc>,
	till_date: &chrono::DateTime<Utc>,
) -> Result<u64, Error> {
	let secrets_usages = db::get_all_secrets_usages(
		&mut *connection,
		&workspace_id,
		&DateTime::from(month_start_date.clone()),
	)
	.await?;

	let mut secrets_bill = 0;
	for secrets_usage in secrets_usages {
		let hours = (secrets_usage
			.stop_time
			.map(|st| chrono::DateTime::from(st))
			.unwrap_or(till_date.clone()) -
			chrono::DateTime::from(secrets_usage.start_time))
		.num_hours();
		let monthly_price = if secrets_usage.secret_count <= 3 {
			0
		} else {
			(secrets_usage.secret_count as f64 / 100f64).ceil() as u64 * 10
		};
		secrets_bill += if hours > 720 {
			monthly_price
		} else {
			hours as u64 * (monthly_price / 720)
		};
	}

	Ok(secrets_bill)
}

pub async fn get_docker_repo_usage_and_bill(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	month_start_date: &chrono::DateTime<Utc>,
	till_date: &chrono::DateTime<Utc>,
) -> Result<u64, Error> {
	let docker_repository_usages = db::get_all_docker_repository_usages(
		&mut *connection,
		&workspace_id,
		&DateTime::from(month_start_date.clone()),
	)
	.await?;

	let mut docker_repository_bill = 0;

	for docker_repository_usage in docker_repository_usages {
		let hours = (docker_repository_usage
			.stop_time
			.map(|st| chrono::DateTime::from(st))
			.unwrap_or(till_date.clone()) -
			chrono::DateTime::from(docker_repository_usage.start_time))
		.num_hours();
		let monthly_price = if docker_repository_usage.storage <= 10 {
			0
		} else if docker_repository_usage.storage <= 10 {
			10
		} else {
			(docker_repository_usage.storage as f64 * 0.1f64).ceil() as u64
		};
		docker_repository_bill += if hours > 720 {
			monthly_price
		} else {
			hours as u64 * (monthly_price / 720)
		};
	}
}

pub async fn get_domain_usage_and_bill(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	month_start_date: &chrono::DateTime<Utc>,
	till_date: &chrono::DateTime<Utc>,
) -> Result<u64, Error> {
	let domains_usages = db::get_all_domains_usages(
		&mut *connection,
		&workspace_id,
		&DateTime::from(month_start_date.clone()),
	)
	.await?;

	let mut domains_bill = 0;
	for domains_usage in domains_usages {
		let hours = (domains_usage
			.stop_time
			.map(|st| chrono::DateTime::from(st))
			.unwrap_or(till_date.clone()) -
			chrono::DateTime::from(domains_usage.start_time))
		.num_hours();
		let monthly_price = match domains_usage.domain_plan {
			DomainPlan::Free => 0,
			DomainPlan::Unlimited => 10,
		};
		domains_bill += if hours > 720 {
			monthly_price
		} else {
			hours as u64 * (monthly_price / 720)
		};
	}

	Ok(domains_bill)
}
