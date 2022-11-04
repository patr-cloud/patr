use std::{
	cmp::{max, min},
	collections::{BTreeMap, HashMap},
	str::FromStr,
};

use api_models::{
	models::workspace::{
		billing::{
			DatabaseUsage,
			DeploymentBills,
			DeploymentUsage,
			DockerRepositoryUsage,
			DomainPlan,
			DomainUsage,
			ManagedUrlUsage,
			PaymentMethod,
			PaymentStatus,
			SecretUsage,
			StaticSitePlan,
			StaticSiteUsage,
			StripePaymentMethodType,
			TransactionType,
			WorkspaceBillBreakdown,
		},
		infrastructure::list_all_deployment_machine_type::DeploymentMachineType,
	},
	utils::{DateTime, PriceAmount, Uuid},
};
use chrono::{Datelike, TimeZone, Utc};
use eve_rs::AsError;
use stripe::{
	Client,
	CreatePaymentIntent,
	CreateRefund,
	CreateSetupIntent,
	Currency,
	CustomerId,
	OffSessionOther,
	PaymentIntent,
	PaymentIntentConfirmationMethod,
	PaymentIntentOffSession,
	PaymentIntentStatus,
	PaymentMethodId,
	Refund,
	SetupIntent,
};

use crate::{
	db::{
		self,
		DomainPlan as DbDomainPlan,
		ManagedDatabasePlan,
		StaticSitePlan as DbStaticSitePlan,
	},
	error,
	models::{
		billing::{
			DatabaseBill,
			DeploymentBill,
			DeploymentBillItem,
			DockerRepositoryBill,
			DomainBill,
			ManagedUrlBill,
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
	let workspace = db::get_workspace_info(connection, workspace_id)
		.await?
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	let default_payment_method_id = workspace
		.default_payment_method_id
		.status(402)
		.body(error!(PAYMENT_METHOD_REQUIRED).to_string())?;

	let address_id = workspace
		.address_id
		.status(400)
		.body(error!(ADDRESS_REQUIRED).to_string())?;

	let (dollars, description) = (credits, "Patr charge: Additional credits");

	let (currency, amount) = if db::get_billing_address(connection, &address_id)
		.await?
		.status(500)?
		.country == *"IN"
	{
		(Currency::INR, (dollars * 100 * 80) as i64)
	} else {
		(Currency::USD, (dollars * 100) as i64)
	};

	let client = Client::new(&config.stripe.secret_key);

	let payment_intent = PaymentIntent::create(&client, {
		let mut intent = CreatePaymentIntent::new(amount, currency);

		intent.confirm = Some(true);
		intent.confirmation_method =
			Some(PaymentIntentConfirmationMethod::Automatic);
		intent.off_session =
			Some(PaymentIntentOffSession::Other(OffSessionOther::OneOff));
		intent.description = Some(description);
		intent.customer =
			Some(CustomerId::from_str(&workspace.stripe_customer_id)?);
		intent.payment_method =
			Some(PaymentMethodId::from_str(&default_payment_method_id)?);
		intent.payment_method_types = Some(vec!["card".to_string()]);

		intent
	})
	.await;

	// handling errors explicitely as `?` doesn't provide enough information
	let payment_intent = match payment_intent {
		Ok(payment) => payment,
		Err(err) => {
			log::error!("Error from stripe: {}", err);
			return Error::as_result()
				.status(500)
				.body(error!(SERVER_ERROR).to_string())?;
		}
	};

	if payment_intent.status != PaymentIntentStatus::Succeeded {
		Refund::create(&client, {
			let mut refund = CreateRefund::new();

			refund.payment_intent = Some(payment_intent.id);

			refund
		})
		.await?;
		return Err(Error::empty()
			.status(400)
			.body(error!(PAYMENT_FAILED).to_string()));
	}

	let transaction_id = db::generate_new_transaction_id(connection).await?;
	let date = Utc::now();

	db::create_transaction(
		connection,
		workspace_id,
		&transaction_id,
		date.month() as i32,
		credits.into(),
		Some(&payment_intent.id),
		&date,
		&TransactionType::Credits,
		&PaymentStatus::Success,
		Some(description),
	)
	.await?;

	Ok(())
}

pub async fn make_payment(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	amount_to_pay: u32,
	config: &Settings,
) -> Result<Uuid, Error> {
	let workspace = db::get_workspace_info(connection, workspace_id)
		.await?
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	let default_payment_method_id = workspace
		.default_payment_method_id
		.status(402)
		.body(error!(PAYMENT_METHOD_REQUIRED).to_string())?;

	let address_id = workspace
		.address_id
		.status(400)
		.body(error!(ADDRESS_REQUIRED).to_string())?;

	let (dollars, description) = (amount_to_pay, "Patr charge: Resource Usage");

	let (currency, amount) = if db::get_billing_address(connection, &address_id)
		.await?
		.status(500)?
		.country == *"IN"
	{
		(Currency::INR, (dollars * 100 * 80) as i64)
	} else {
		(Currency::USD, (dollars * 100) as i64)
	};

	let client = Client::new(&config.stripe.secret_key);

	let payment_intent = PaymentIntent::create(&client, {
		let mut intent = CreatePaymentIntent::new(amount, currency);

		intent.confirm = Some(false);
		intent.description = Some(description);
		intent.customer =
			Some(CustomerId::from_str(&workspace.stripe_customer_id)?);
		intent.payment_method =
			Some(PaymentMethodId::from_str(&default_payment_method_id)?);
		intent.payment_method_types = Some(vec!["card".to_string()]);

		intent
	})
	.await;

	// handling errors explicitely as `?` doesn't provide enough information
	let payment_intent = match payment_intent {
		Ok(payment) => payment,
		Err(err) => {
			log::error!("Error from stripe: {}", err);
			return Error::as_result()
				.status(500)
				.body(error!(SERVER_ERROR).to_string())?;
		}
	};

	let transaction_id = db::generate_new_transaction_id(connection).await?;
	let date = Utc::now();

	db::create_transaction(
		connection,
		workspace_id,
		&transaction_id,
		date.month() as i32,
		amount_to_pay.into(),
		Some(&payment_intent.id),
		&date,
		&TransactionType::Payment,
		&PaymentStatus::Pending,
		Some(description),
	)
	.await?;

	Ok(transaction_id)
}

pub async fn confirm_payment_method(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	transaction_id: &Uuid,
	config: &Settings,
) -> Result<bool, Error> {
	let transaction = db::get_transaction_by_transaction_id(
		connection,
		workspace_id,
		transaction_id,
	)
	.await?
	.status(404)
	.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	if let Some(payment_intent_id) = transaction.payment_intent_id {
		let client = Client::new(&config.stripe.secret_key);
		let payment_intent = PaymentIntent::confirm(
			&client,
			&payment_intent_id,
			Default::default(),
		)
		.await?;

		if payment_intent.status != PaymentIntentStatus::Succeeded {
			Refund::create(&client, {
				let mut refund = CreateRefund::new();

				refund.payment_intent = Some(payment_intent.id);

				refund
			})
			.await?;
			db::update_transaction_status(
				connection,
				transaction_id,
				&PaymentStatus::Failed,
			)
			.await?;
			return Ok(false);
		}
		db::update_transaction_status(
			connection,
			transaction_id,
			&PaymentStatus::Success,
		)
		.await?;
	} else {
		log::trace!(
			"payment_intent_id not found for transaction_id: {}, this is a not an expected behaviour", transaction_id);
		return Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;
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
		till_date,
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
				.map(|deployment| deployment.name)
				.as_deref()
				.unwrap_or("unknown")
				.to_string(),
				bill_items: vec![],
			})
			.bill_items
			.push(DeploymentBillItem {
				machine_type: DeploymentMachineType {
					id: deployment_usage.machine_type,
					cpu_count: *cpu_count,
					memory_count: *memory_count,
				},
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
		till_date,
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
					.map(|database| database.name)
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
) -> Result<HashMap<DbStaticSitePlan, StaticSiteBill>, Error> {
	let static_sites_usages = db::get_all_static_site_usages(
		&mut *connection,
		workspace_id,
		&DateTime::from(*month_start_date),
		till_date,
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
			DbStaticSitePlan::Free => 0f64,
			DbStaticSitePlan::Pro => 5f64,
			DbStaticSitePlan::Unlimited => 10f64,
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
		till_date,
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
		till_date,
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
) -> Result<HashMap<DbDomainPlan, DomainBill>, Error> {
	let domains_usages = db::get_all_domains_usages(
		&mut *connection,
		workspace_id,
		&DateTime::from(*month_start_date),
		till_date,
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
			DbDomainPlan::Free => 0f64,
			DbDomainPlan::Unlimited => 10f64,
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
		till_date,
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
) -> Result<SetupIntent, Error> {
	let workspace = db::get_workspace_info(connection, workspace_id)
		.await?
		.status(500)?;

	if workspace.address_id.is_none() {
		return Error::as_result()
			.status(400)
			.body(error!(ADDRESS_REQUIRED).to_string())?;
	}

	SetupIntent::create(&Client::new(&config.stripe.secret_key), {
		let mut intent = CreateSetupIntent::new();

		intent.customer =
			Some(CustomerId::from_str(workspace.stripe_customer_id.as_str())?);
		intent.payment_method_types = Some(vec!["card".to_string()]);

		intent
	})
	.await
	.map_err(|err| err.into())
}

pub async fn get_card_details(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	config: &Settings,
) -> Result<Vec<PaymentMethod>, Error> {
	let payment_source_list =
		db::get_payment_methods_for_workspace(connection, workspace_id).await?;

	let mut cards = Vec::new();
	let client = Client::new(&config.stripe.secret_key);
	for payment_source in payment_source_list {
		let card_details = stripe::PaymentMethod::retrieve(
			&client,
			&PaymentMethodId::from_str(&payment_source.payment_method_id)?,
			&[],
		)
		.await?;

		let card_details = PaymentMethod {
			id: card_details.id.to_string(),
			customer: card_details.customer.status(500)?.id().to_string(),
			r#type: match card_details.type_ {
				stripe::PaymentMethodType::Card => {
					StripePaymentMethodType::CardPresent
				}
				stripe::PaymentMethodType::CardPresent => {
					StripePaymentMethodType::CardPresent
				}
				_ => {
					return Error::as_result()
						.status(500)
						.body(error!(SERVER_ERROR).to_string())?
				}
			},
			card: serde_json::from_str(&serde_json::to_string(
				&card_details.card.status(500)?,
			)?)?,
			created: DateTime::from(Utc.timestamp_millis(card_details.created)),
		};
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

	let client = Client::new(&config.stripe.secret_key);
	stripe::PaymentMethod::detach(
		&client,
		&PaymentMethodId::from_str(payment_method_id)?,
	)
	.await?;

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

pub async fn get_total_resource_usage(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	month_start_date: &chrono::DateTime<Utc>,
	till_date: &chrono::DateTime<Utc>,
	year: u32,
	month: u32,
) -> Result<WorkspaceBillBreakdown, Error> {
	let deployment_usage = calculate_deployment_bill_for_workspace_till(
		connection,
		workspace_id,
		month_start_date,
		till_date,
	)
	.await?
	.into_iter()
	.map(|(key, value)| {
		(
			key,
			DeploymentUsage {
				name: value.deployment_name,
				bill_items: value
					.bill_items
					.into_iter()
					.map(|bill_item| DeploymentBills {
						machine_type: DeploymentMachineType {
							id: bill_item.machine_type.id,
							cpu_count: bill_item.machine_type.cpu_count,
							memory_count: bill_item.machine_type.memory_count,
						},
						num_instances: bill_item.num_instances,
						hours: bill_item.hours,
						amount: PriceAmount(bill_item.amount),
					})
					.collect::<Vec<_>>(),
			},
		)
	})
	.collect::<BTreeMap<_, _>>();
	let deployment_charge = PriceAmount(
		deployment_usage
			.iter()
			.map(|(_, usage)| usage.bill_items.iter())
			.flatten()
			.map(|item| item.amount.0)
			.sum(),
	);

	let database_usage = calculate_database_bill_for_workspace_till(
		connection,
		workspace_id,
		month_start_date,
		till_date,
	)
	.await?
	.into_iter()
	.map(|(key, value)| {
		(
			key,
			DatabaseUsage {
				name: value.database_name,
				hours: value.hours,
				amount: PriceAmount(value.amount),
			},
		)
	})
	.collect::<BTreeMap<_, _>>();
	let database_charge = PriceAmount(
		database_usage.iter().map(|(_, usage)| usage.amount.0).sum(),
	);

	let static_site_usage = calculate_static_sites_bill_for_workspace_till(
		connection,
		workspace_id,
		month_start_date,
		till_date,
	)
	.await?
	.into_iter()
	.map(|(key, value)| {
		(
			match key {
				DbStaticSitePlan::Pro => StaticSitePlan::Pro,
				DbStaticSitePlan::Free => StaticSitePlan::Free,
				DbStaticSitePlan::Unlimited => StaticSitePlan::Unlimited,
			},
			StaticSiteUsage {
				hours: value.hours,
				amount: PriceAmount(value.amount),
			},
		)
	})
	.collect::<BTreeMap<_, _>>();
	let static_site_charge = PriceAmount(
		static_site_usage
			.iter()
			.map(|(_, usage)| usage.amount.0)
			.sum(),
	);

	let managed_url_usage = calculate_managed_urls_bill_for_workspace_till(
		connection,
		workspace_id,
		month_start_date,
		till_date,
	)
	.await?
	.into_iter()
	.map(|(key, value)| {
		(
			key as u32,
			ManagedUrlUsage {
				hours: value.hours,
				amount: PriceAmount(value.amount),
			},
		)
	})
	.collect::<BTreeMap<_, _>>();
	let managed_url_charge = PriceAmount(
		managed_url_usage
			.iter()
			.map(|(_, usage)| usage.amount.0)
			.sum(),
	);

	let docker_repository_usage =
		calculate_docker_repository_bill_for_workspace_till(
			connection,
			workspace_id,
			month_start_date,
			till_date,
		)
		.await?
		.into_iter()
		.map(|docker_repo_usage| DockerRepositoryUsage {
			storage: docker_repo_usage.storage as u32,
			hours: docker_repo_usage.hours,
			amount: PriceAmount(docker_repo_usage.amount),
		})
		.collect::<Vec<_>>();
	let docker_repository_charge = PriceAmount(
		docker_repository_usage
			.iter()
			.map(|usage| usage.amount.0)
			.sum(),
	);

	let domain_usage = calculate_domains_bill_for_workspace_till(
		connection,
		workspace_id,
		month_start_date,
		till_date,
	)
	.await?
	.into_iter()
	.map(|(key, value)| {
		(
			match key {
				DbDomainPlan::Free => DomainPlan::Free,
				DbDomainPlan::Unlimited => DomainPlan::Unlimited,
			},
			DomainUsage {
				hours: value.hours,
				amount: PriceAmount(value.amount),
			},
		)
	})
	.collect::<BTreeMap<_, _>>();
	let domain_charge =
		PriceAmount(domain_usage.iter().map(|(_, usage)| usage.amount.0).sum());

	let secret_usage = calculate_secrets_bill_for_workspace_till(
		connection,
		workspace_id,
		month_start_date,
		till_date,
	)
	.await?
	.into_iter()
	.map(|(key, value)| {
		(
			key as u32,
			SecretUsage {
				hours: value.hours,
				amount: PriceAmount(value.amount),
			},
		)
	})
	.collect::<BTreeMap<_, _>>();
	let secret_charge =
		PriceAmount(secret_usage.iter().map(|(_, usage)| usage.amount.0).sum());

	let total_charge = PriceAmount(
		deployment_charge.0 +
			database_charge.0 +
			static_site_charge.0 +
			managed_url_charge.0 +
			domain_charge.0 +
			secret_charge.0 +
			docker_repository_charge.0,
	);

	let bill = WorkspaceBillBreakdown {
		year,
		month,
		total_charge,
		deployment_charge,
		deployment_usage,
		database_charge,
		database_usage,
		static_site_charge,
		static_site_usage,
		domain_charge,
		domain_usage,
		managed_url_charge,
		managed_url_usage,
		secret_charge,
		secret_usage,
		docker_repository_charge,
		docker_repository_usage,
	};
	Ok(bill)
}
