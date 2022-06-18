use api_models::utils::{DateTime, Uuid};
use chrono::Utc;
use eve_rs::AsError;
use reqwest::Client;
use serde_json::json;

use crate::{
	db,
	error,
	models::{PaymentIntent, PaymentIntentObject, PaymentMethodStatus},
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
			amount: credits * 10,
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

	let db_usages = db::get_all_database_usage(
		&mut *connection,
		&workspace_id,
		&DateTime::from(month_start_date.clone()),
	)
	.await?;

	let static_site_usages = db::get_all_static_site_usages(
		&mut *connection,
		&workspace_id,
		&DateTime::from(month_start_date.clone()),
	)
	.await?;

	let managed_url_usages = db::get_all_managed_url_usages(
		&mut *connection,
		&workspace_id,
		&DateTime::from(month_start_date.clone()),
	)
	.await?;

	let docker_repository_usages = db::get_all_docker_repository_usages(
		&mut *connection,
		&workspace_id,
		&DateTime::from(month_start_date.clone()),
	)
	.await?;

	let domains_usages = db::get_all_domains_usages(
		&mut *connection,
		&workspace_id,
		&DateTime::from(month_start_date.clone()),
	)
	.await?;

	let secrets_usages = db::get_all_secrets_usages(
		&mut *connection,
		&workspace_id,
		&DateTime::from(month_start_date.clone()),
	)
	.await?;

	let mut deployment_bill = 0;
	for deployment_usage in deployment_usages {
		let hours = (deployment_usage.stop_time.into().unwrap_or(till_date) -
			deployment_usage.start_time)
			.num_hours();
		let monthly_price = match deployment_usage.plan {
			DeploymentPlan::Nano => {
				if deployment_usage.num_instance == 1 {
					// TODO free deployment
					0
				} else {
					5
				}
			}
			DeploymentPlan::Micro => 10,
			DeploymentPlan::Small => 20,
			DeploymentPlan::Medium => 40,
			DeploymentPlan::Large => 80,
		};
		deployment_bill += if hours > 720 {
			monthly_price
		} else {
			hours * (monthly_price / 720)
		};
	}

	let mut database_bill = 0;
	for database_usage in database_usages {
		let hours = (database_usage.stop_time.unwrap_or(till_date) -
			database_usage.start_time)
			.num_hours();
		let monthly_price = match database_usage.plan {
			// TODO update this
			DatabasePlan::Nano => 5,
			DatabasePlan::Micro => 10,
			DatabasePlan::Small => 20,
			DatabasePlan::Medium => 40,
			DatabasePlan::Large => 80,
		};
		database_bill += if hours > 720 {
			monthly_price
		} else {
			hours * (monthly_price / 720)
		};
	}

	let mut static_sites_bill = 0;
	for static_sites_usage in static_sites_usages {
		let hours = (static_sites_usage.stop_time.unwrap_or(till_date) -
			static_sites_usage.start_time)
			.num_hours();
		let monthly_price = match static_sites_usage.plan {
			StaticSitesPlan::Free => 0,
			StaticSitesPlan::Pro => 5,
			StaticSitesPlan::Unlimited => 10,
		};
		static_sites_bill += if hours > 720 {
			monthly_price
		} else {
			hours * (monthly_price / 720)
		};
	}

	let mut managed_url_bill = 0;
	for managed_url_usage in managed_url_usages {
		let hours = (managed_url_usage.stop_time.unwrap_or(till_date) -
			managed_url_usage.start_time)
			.num_hours();
		let monthly_price = if managed_url_usage.url_count <= 10 {
			0
		} else {
			(managed_url_usage.url_count / 100).ceil() * 10
		};
		managed_url_bill += if hours > 720 {
			monthly_price
		} else {
			hours * (monthly_price / 720)
		};
	}

	let mut docker_repository_bill = 0;
	for docker_repository_usage in docker_repository_usages {
		let hours = (docker_repository_usage.stop_time.unwrap_or(till_date) -
			docker_repository_usage.start_time)
			.num_hours();
		let monthly_price = if docker_repository_usage.storage <= 10 {
			0
		} else if docker_repository_usage.storage <= 10 {
			10
		} else {
			docker_repository_usage.storage * 0.1f64;
		};
		docker_repository_bill += if hours > 720 {
			monthly_price
		} else {
			hours * (monthly_price / 720)
		};
	}

	let mut domains_bill = 0;
	for domains_usage in domains_usages {
		let hours = (domains_usage.stop_time.unwrap_or(till_date) -
			domains_usage.start_time)
			.num_hours();
		let monthly_price = match domains_usage.domain_plan {
			DomainPlan::Free => 0,
			DomainPlan::Unlimited => 10,
		};
		domains_bill += if hours > 720 {
			monthly_price
		} else {
			hours * (monthly_price / 720)
		};
	}

	let mut secrets_bill = 0;
	for secrets_usage in secrets_usages {
		let hours = (secrets_usage.stop_time.unwrap_or(till_date) -
			secrets_usage.start_time)
			.num_hours();
		let monthly_price = if secrets_usage.secret_count <= 3 {
			0
		} else {
			(secrets_usage.secret_count / 100).ceil() * 10
		};
		secrets_bill += if hours > 720 {
			monthly_price
		} else {
			hours * (monthly_price / 720)
		};
	}

	Ok(deployment_bill +
		database_bill +
		static_sites_bill +
		managed_url_bill +
		docker_repository_bill +
		domains_bill +
		secrets_bill)
}
