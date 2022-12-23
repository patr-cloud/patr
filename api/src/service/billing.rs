use std::{
	cmp::{max, min},
	collections::HashMap,
	str::FromStr,
};

use api_models::{
	models::workspace::{
		billing::{
			DatabaseUsage,
			DeploymentBill,
			DeploymentUsage,
			DockerRepositoryUsage,
			DomainPlan,
			DomainUsage,
			ManagedUrlUsage,
			PatrDatabaseUsage,
			PaymentMethod,
			PaymentStatus,
			SecretUsage,
			StaticSitePlan,
			StaticSiteUsage,
			TransactionType,
			VolumeUsage,
			WorkspaceBillBreakdown,
		},
		infrastructure::{
			database::{PatrDatabasePlan, PatrDatabaseStatus},
			deployment::DeploymentStatus,
			list_all_deployment_machine_type::DeploymentMachineType,
		},
	},
	utils::{DateTime, Uuid},
};
use chrono::{Datelike, TimeZone, Utc};
use eve_rs::AsError;
use stripe::{
	Client,
	CreatePaymentIntent,
	CreateRefund,
	Currency,
	CustomerId,
	PaymentIntent,
	PaymentIntentId,
	PaymentIntentSetupFutureUsage,
	PaymentIntentStatus,
	PaymentMethodId,
	Refund,
};

use crate::{
	db::{
		self,
		DomainPlan as DbDomainPlan,
		ManagedDatabasePlan,
		ManagedDatabaseStatus,
		StaticSitePlan as DbStaticSitePlan,
		Workspace,
	},
	error,
	models::{deployment, IpQualityScore},
	utils::{settings::Settings, Error},
	Database,
};

pub async fn add_credits_to_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	credits: u64,
	payment_method_id: &str,
	config: &Settings,
) -> Result<(Uuid, String), Error> {
	let workspace = db::get_workspace_info(connection, workspace_id)
		.await?
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	let is_payment_method_valid =
		db::get_payment_methods_for_workspace(connection, workspace_id)
			.await?
			.iter()
			.any(|payment_method| {
				payment_method.payment_method_id == payment_method_id
			});
	if !is_payment_method_valid {
		return Err(Error::empty()
			.status(400)
			.body(error!(INVALID_PAYMENT_METHOD).to_string()));
	};

	let address_id = workspace
		.address_id
		.status(400)
		.body(error!(ADDRESS_REQUIRED).to_string())?;

	let (cents, description) = (credits, "Patr charge: Additional credits");

	let (currency, amount) = if db::get_billing_address(connection, &address_id)
		.await?
		.status(500)?
		.country == *"IN"
	{
		(Currency::INR, cents * 80)
	} else {
		(Currency::USD, cents)
	};

	let client = Client::new(&config.stripe.secret_key);

	let payment_intent = PaymentIntent::create(&client, {
		let mut intent = CreatePaymentIntent::new(amount as i64, currency);

		intent.confirm = Some(false);
		intent.description = Some(description);
		intent.customer =
			Some(CustomerId::from_str(&workspace.stripe_customer_id)?);
		intent.payment_method =
			Some(PaymentMethodId::from_str(payment_method_id)?);
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
		credits,
		Some(&payment_intent.id),
		&date,
		&TransactionType::Credits,
		&PaymentStatus::Pending,
		Some(description),
	)
	.await?;

	let client_secret = payment_intent.client_secret.ok_or_else(|| {
		log::error!("PaymentIntent does not have a client secret");
		Error::empty()
			.status(400)
			.body(error!(INVALID_PAYMENT_METHOD).to_string())
	})?;

	Ok((transaction_id, client_secret))
}

pub async fn verify_card_with_charges(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace: &Workspace,
	payment_intent_id: &str,
	config: &Settings,
) -> Result<(), Error> {
	let client = Client::new(&config.stripe.secret_key);
	let payment_intent = PaymentIntent::retrieve(
		&client,
		&PaymentIntentId::from_str(payment_intent_id)?,
		Default::default(),
	)
	.await?;

	let payment_method_id =
		if let Some(payment_method) = payment_intent.clone().payment_method {
			payment_method.id()
		} else {
			return Error::as_result().status(500)?;
		};

	match payment_intent.status {
		PaymentIntentStatus::Succeeded => {
			db::add_payment_method_info(
				connection,
				&workspace.id,
				&payment_method_id,
			)
			.await?;

			if workspace.default_payment_method_id.is_none() {
				db::set_default_payment_method_for_workspace(
					connection,
					&workspace.id,
					&payment_method_id,
				)
				.await?;
			}

			db::unmark_workspace_as_spam(connection, &workspace.id).await?;

			Refund::create(&client, {
				let mut refund = CreateRefund::new();

				// https://stripe.com/docs/api/refunds#create_refund
				refund.payment_intent =
					Some(PaymentIntentId::from_str(payment_intent_id)?);

				refund
			})
			.await?;

			Ok(())
		}
		PaymentIntentStatus::Canceled => Error::as_result()
			.status(500)
			.body(error!(PAYMENT_FAILED).to_string())?,
		_ => {
			log::info!("Payment is still in pending state, make the payment and then confirm again");
			Error::as_result()
				.status(500)
				.body(error!(PAYMENT_PENDING).to_string())?
		}
	}
}

pub async fn make_payment(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	amount_to_pay_in_cents: u64,
	payment_method_id: &str,
	config: &Settings,
) -> Result<(Uuid, String), Error> {
	let workspace = db::get_workspace_info(connection, workspace_id)
		.await?
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	let is_payment_method_valid =
		db::get_payment_methods_for_workspace(connection, workspace_id)
			.await?
			.iter()
			.any(|payment_method| {
				payment_method.payment_method_id == payment_method_id
			});
	if !is_payment_method_valid {
		return Err(Error::empty()
			.status(400)
			.body(error!(INVALID_PAYMENT_METHOD).to_string()));
	};

	let address_id = workspace
		.address_id
		.status(400)
		.body(error!(ADDRESS_REQUIRED).to_string())?;

	let (cents, description) =
		(amount_to_pay_in_cents, "Patr charge: Resource Usage");

	let (currency, amount) = if db::get_billing_address(connection, &address_id)
		.await?
		.status(500)?
		.country == *"IN"
	{
		(Currency::INR, cents * 80)
	} else {
		(Currency::USD, cents)
	};

	let client = Client::new(&config.stripe.secret_key);

	let payment_intent = PaymentIntent::create(&client, {
		let mut intent = CreatePaymentIntent::new(amount as i64, currency);

		intent.confirm = Some(false);
		intent.description = Some(description);
		intent.customer =
			Some(CustomerId::from_str(&workspace.stripe_customer_id)?);
		intent.payment_method =
			Some(PaymentMethodId::from_str(payment_method_id)?);
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
		amount_to_pay_in_cents,
		Some(&payment_intent.id),
		&date,
		&TransactionType::Payment,
		&PaymentStatus::Pending,
		Some(description),
	)
	.await?;

	let client_secret = payment_intent.client_secret.ok_or_else(|| {
		log::error!("PaymentIntent does not have a client secret");
		Error::empty()
			.status(400)
			.body(error!(INVALID_PAYMENT_METHOD).to_string())
	})?;

	Ok((transaction_id, client_secret))
}

pub async fn confirm_payment(
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
		let payment_intent = PaymentIntent::retrieve(
			&client,
			&PaymentIntentId::from_str(&payment_intent_id)?,
			Default::default(),
		)
		.await?;

		match payment_intent.status {
			PaymentIntentStatus::Succeeded => {
				db::update_transaction_status(
					connection,
					transaction_id,
					&PaymentStatus::Success,
				)
				.await?;

				Ok(true)
			}
			PaymentIntentStatus::Canceled => {
				db::update_transaction_status(
					connection,
					transaction_id,
					&PaymentStatus::Failed,
				)
				.await?;

				Ok(false)
			}
			_ => {
				log::info!("Payment is still in pending state, make the payment and then confirm again");
				Ok(false)
			}
		}
	} else {
		log::trace!(
			"payment_intent_id not found for transaction_id: {}, this is a not an expected behaviour", transaction_id);
		Err(Error::empty()
			.status(500)
			.body(error!(SERVER_ERROR).to_string()))
	}
}

pub async fn calculate_deployment_bill_for_workspace_till(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	month_start_date: &chrono::DateTime<Utc>,
	till_date: &chrono::DateTime<Utc>,
) -> Result<Vec<DeploymentUsage>, Error> {
	let deployment_usages = db::get_all_deployment_usage(
		&mut *connection,
		workspace_id,
		&DateTime::from(*month_start_date),
		till_date,
	)
	.await?;

	let mut deployment_usage_bill = HashMap::<Uuid, Vec<DeploymentBill>>::new();
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

		let price_in_dollars = if (*cpu_count, *memory_count) == (1, 2) &&
			deployment_usage.num_instance == 1
		{
			// Free eligible
			let price = (((hours - remaining_free_hours).max(0) as f64) /
				720f64) * monthly_price;
			remaining_free_hours = (remaining_free_hours - hours).max(0);
			price
		} else if hours >= 720 {
			monthly_price
		} else {
			((hours as f64) / 720f64) * monthly_price
		};

		let price_in_cents = (price_in_dollars * 100.0).round() as u64;

		deployment_usage_bill
			.entry(deployment_usage.deployment_id)
			.or_default()
			.push(DeploymentBill {
				start_time: DateTime(start_time),
				stop_time: deployment_usage.stop_time.map(DateTime),
				machine_type: DeploymentMachineType {
					id: deployment_usage.machine_type,
					cpu_count: *cpu_count,
					memory_count: *memory_count,
				},
				num_instances: deployment_usage.num_instance as u32,
				hours: hours as u64,
				amount: price_in_cents,
				monthly_charge: monthly_price as u64 * 100,
			});
	}

	let mut result = vec![];
	for (deployment_id, deployment_bill) in deployment_usage_bill {
		// safe to unwrap Option<T> as if deployment_usage is present then that
		// deployment id should be in db
		let deployment = db::get_deployment_by_id_including_deleted(
			connection,
			&deployment_id,
		)
		.await?
		.status(500)?;

		result.push(DeploymentUsage {
			name: deployment.name,
			is_deleted: deployment.status == DeploymentStatus::Deleted,
			deployment_id,
			deployment_bill,
		})
	}

	Ok(result)
}

pub async fn calculate_volumes_bill_for_workspace_till(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	month_start_date: &chrono::DateTime<Utc>,
	till_date: &chrono::DateTime<Utc>,
) -> Result<Vec<VolumeUsage>, Error> {
	let volume_usages = db::get_all_volume_usage(
		&mut *connection,
		workspace_id,
		&DateTime::from(*month_start_date),
		till_date,
	)
	.await?;

	let mut volume_usage_bill = Vec::new();

	for volume_usages in volume_usages {
		let stop_time = volume_usages
			.stop_time
			.map(chrono::DateTime::from)
			.unwrap_or_else(|| *till_date);
		let start_time = max(volume_usages.start_time, *month_start_date);
		let hours = min(
			720,
			((stop_time - start_time).num_seconds() as f64 / 3600f64).ceil()
				as i64,
		);

		let storage_in_gb = (volume_usages.storage as f64 /
			(1024 * 1024 * 1024) as f64)
			.ceil() as i64;
		let monthly_price = (storage_in_gb as f64 * 0.1f64).ceil();

		let price_in_dollars = if hours > 720 {
			monthly_price
		} else {
			(hours as f64 / 720f64) * monthly_price
		};

		let price_in_cents = (price_in_dollars * 100.0).round() as u64 *
			volume_usages.number_of_volumes as u64;

		volume_usage_bill.push(VolumeUsage {
			start_time: DateTime(start_time),
			stop_time: volume_usages.stop_time.map(DateTime),
			storage: volume_usages.storage as u64,
			number_of_volume: volume_usages.number_of_volumes as u32,
			hours: hours as u64,
			amount: price_in_cents,
			monthly_charge: monthly_price as u64 * 100,
		})
	}

	Ok(volume_usage_bill)
}

pub async fn calculate_patr_database_bill_for_workspace_till(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	month_start_date: &chrono::DateTime<Utc>,
	till_date: &chrono::DateTime<Utc>,
) -> Result<Vec<PatrDatabaseUsage>, Error> {
	let database_usages = db::get_all_patr_database_usage(
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

		// todo: revisit patr db pricing
		let monthly_price = match database_usage.db_plan {
			PatrDatabasePlan::db_1r_1c_10v => 5f64,
			PatrDatabasePlan::db_2r_2c_25v => 10f64,
		};

		let price_in_dollars = if hours >= 720 {
			monthly_price
		} else {
			(hours as f64 / 720f64) * monthly_price
		};

		let price_in_cents = (price_in_dollars * 100.0).round() as u64;

		let managed_database = db::get_patr_database_by_id_including_deleted(
			connection,
			&database_usage.database_id,
		)
		.await?
		.status(500)?; // If database_id is presnt in usage table the it should be in the
			   // database table as well, otherwise this is something wrong with our
			   // logic

		database_usage_bill
			.entry(database_usage.database_id.clone())
			.or_insert(PatrDatabaseUsage {
				start_time: DateTime(start_time),
				deletion_time: database_usage.deletion_time.map(DateTime),
				database_id: database_usage.database_id.clone(),
				name: managed_database.name,
				hours: hours as u64,
				amount: price_in_cents,
				is_deleted: managed_database.status ==
					PatrDatabaseStatus::Deleted,
				monthly_charge: monthly_price as u64 * 100,
				plan: database_usage.db_plan.to_string(),
			});
	}

	Ok(database_usage_bill.into_iter().map(|(_, v)| v).collect())
}

pub async fn calculate_database_bill_for_workspace_till(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	month_start_date: &chrono::DateTime<Utc>,
	till_date: &chrono::DateTime<Utc>,
) -> Result<Vec<DatabaseUsage>, Error> {
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

		let price_in_dollars = if hours >= 720 {
			monthly_price
		} else {
			(hours as f64 / 720f64) * monthly_price
		};

		let price_in_cents = (price_in_dollars * 100.0).round() as u64;

		let managed_database =
			db::get_managed_database_by_id_including_deleted(
				connection,
				&database_usage.database_id,
			)
			.await?
			.status(500)?; // If database_id is presnt in usage table the it should be in the
			   // database table as well, otherwise this is something wrong with our
			   // logic

		database_usage_bill
			.entry(database_usage.database_id.clone())
			.or_insert(DatabaseUsage {
				start_time: DateTime(start_time),
				deletion_time: database_usage.deletion_time.map(DateTime),
				database_id: database_usage.database_id.clone(),
				name: managed_database.name,
				hours: hours as u64,
				amount: price_in_cents,
				is_deleted: managed_database.status ==
					ManagedDatabaseStatus::Deleted,
				monthly_charge: monthly_price as u64 * 100,
				plan: database_usage.db_plan.to_string(),
			});
	}

	Ok(database_usage_bill.into_values().collect())
}

pub async fn calculate_static_sites_bill_for_workspace_till(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	month_start_date: &chrono::DateTime<Utc>,
	till_date: &chrono::DateTime<Utc>,
) -> Result<Vec<StaticSiteUsage>, Error> {
	let static_sites_usages = db::get_all_static_site_usages(
		&mut *connection,
		workspace_id,
		&DateTime::from(*month_start_date),
		till_date,
	)
	.await?;

	let mut static_sites_bill: Vec<StaticSiteUsage> = Vec::new();
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
		let price_in_dollars = if hours >= 720 {
			monthly_price
		} else {
			(hours as f64 / 720f64) * monthly_price
		};
		let price_in_cents = (price_in_dollars * 100.0).round() as u64;

		let plan = match static_sites_usage.static_site_plan {
			DbStaticSitePlan::Free => StaticSitePlan::Free,
			DbStaticSitePlan::Pro => StaticSitePlan::Pro,
			DbStaticSitePlan::Unlimited => StaticSitePlan::Unlimited,
		};

		match static_sites_bill.last_mut() {
			Some(last_site_bill) if last_site_bill.plan == plan => {
				let hours = min(
					720,
					((stop_time - last_site_bill.start_time.0).num_seconds()
						as f64 / 3600f64)
						.ceil() as u64,
				);
				let price_in_dollars = if hours >= 720 {
					monthly_price
				} else {
					(hours as f64 / 720f64) * monthly_price
				};
				let price_in_cents = (price_in_dollars * 100.0).round() as u64;

				last_site_bill.stop_time =
					static_sites_usage.stop_time.map(DateTime);
				last_site_bill.hours = hours;
				last_site_bill.amount = price_in_cents;
			}
			_ => {
				static_sites_bill.push(StaticSiteUsage {
					plan,
					hours: hours as u64,
					amount: price_in_cents,
					start_time: DateTime(start_time),
					stop_time: static_sites_usage.stop_time.map(DateTime),
					monthly_charge: monthly_price as u64 * 100,
				});
			}
		};
	}

	Ok(static_sites_bill)
}

pub async fn calculate_managed_urls_bill_for_workspace_till(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	month_start_date: &chrono::DateTime<Utc>,
	till_date: &chrono::DateTime<Utc>,
) -> Result<Vec<ManagedUrlUsage>, Error> {
	let managed_url_usages = db::get_all_managed_url_usages(
		&mut *connection,
		workspace_id,
		&DateTime::from(*month_start_date),
		till_date,
	)
	.await?;

	let mut managed_url_bill: Vec<ManagedUrlUsage> = Vec::new();
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

		let price_in_dollars = if hours >= 720 {
			monthly_price
		} else {
			(hours as f64 / 720f64) * monthly_price
		};
		let price_in_cents = (price_in_dollars * 100.0).round() as u64;

		let urls = managed_url_usage.url_count as f64;
		let urls = (urls / 100f64).ceil() as u64 * 100;
		let plan = if managed_url_usage.url_count <= 10 {
			"0-10 URLs".to_string()
		} else {
			format!("{}-{} URLs", max(11, urls - 100), urls)
		};

		match managed_url_bill.last_mut() {
			Some(last_managed_url) if last_managed_url.plan == plan => {
				let hours = min(
					720,
					((stop_time - last_managed_url.start_time.0).num_seconds()
						as f64 / 3600f64)
						.ceil() as u64,
				);

				let price_in_dollars = if hours >= 720 {
					monthly_price
				} else {
					(hours as f64 / 720f64) * monthly_price
				};
				let price_in_cents = (price_in_dollars * 100.0).round() as u64;

				last_managed_url.stop_time =
					managed_url_usage.stop_time.map(DateTime);
				last_managed_url.hours = hours;
				last_managed_url.amount = price_in_cents;
			}
			_ => {
				managed_url_bill.push(ManagedUrlUsage {
					plan,
					hours: hours as u64,
					amount: price_in_cents,
					start_time: DateTime(start_time),
					stop_time: managed_url_usage.stop_time.map(DateTime),
					monthly_charge: monthly_price as u64 * 100,
				});
			}
		}
	}

	Ok(managed_url_bill)
}

pub async fn calculate_docker_repository_bill_for_workspace_till(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	month_start_date: &chrono::DateTime<Utc>,
	till_date: &chrono::DateTime<Utc>,
) -> Result<Vec<DockerRepositoryUsage>, Error> {
	let docker_repository_usages = db::get_all_docker_repository_usages(
		&mut *connection,
		workspace_id,
		&DateTime::from(*month_start_date),
		till_date,
	)
	.await?;

	let mut docker_repository_bill: Vec<DockerRepositoryUsage> = Vec::new();

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

		let price_in_dollars = if hours >= 720 {
			monthly_price
		} else {
			(hours as f64 / 720f64) * monthly_price
		};
		let price_in_cents = (price_in_dollars * 100.0).round() as u64;

		let plan = if storage_in_gb <= 10 {
			"0-10 GB".to_string()
		} else if storage_in_gb > 10 && storage_in_gb <= 100 {
			"11-100 GB".to_string()
		} else {
			format!("100GB + {}GB additional", storage_in_gb - 100)
		};

		match docker_repository_bill.last_mut() {
			Some(last_repo_bill) if last_repo_bill.plan == plan => {
				let hours = min(
					720,
					((stop_time - last_repo_bill.start_time.0).num_seconds()
						as f64 / 3600f64)
						.ceil() as u64,
				);

				// since both last and current are under same plan,
				// its okay to use any one of those storage size
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

				let price_in_dollars = if hours >= 720 {
					monthly_price
				} else {
					(hours as f64 / 720f64) * monthly_price
				};
				let price_in_cents = (price_in_dollars * 100.0).round() as u64;

				last_repo_bill.stop_time =
					docker_repository_usage.stop_time.map(DateTime);
				last_repo_bill.hours = hours;
				last_repo_bill.amount = price_in_cents;
			}
			_ => {
				docker_repository_bill.push(DockerRepositoryUsage {
					plan,
					hours: hours as u64,
					amount: price_in_cents,
					start_time: DateTime(start_time),
					stop_time: docker_repository_usage.stop_time.map(DateTime),
					monthly_charge: monthly_price as u64 * 100,
				});
			}
		}
	}

	Ok(docker_repository_bill)
}

pub async fn calculate_domains_bill_for_workspace_till(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	month_start_date: &chrono::DateTime<Utc>,
	till_date: &chrono::DateTime<Utc>,
) -> Result<Vec<DomainUsage>, Error> {
	let domains_usages = db::get_all_domains_usages(
		&mut *connection,
		workspace_id,
		&DateTime::from(*month_start_date),
		till_date,
	)
	.await?;

	let mut domains_bill: Vec<DomainUsage> = Vec::new();
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

		let price_in_dollars = if hours >= 720 {
			monthly_price
		} else {
			hours as f64 * (monthly_price / 720f64)
		};
		let price_in_cents = (price_in_dollars * 100.0).round() as u64;

		let plan = match domains_usage.domain_plan {
			DbDomainPlan::Free => DomainPlan::Free,
			DbDomainPlan::Unlimited => DomainPlan::Unlimited,
		};

		match domains_bill.last_mut() {
			Some(last_domain_bill) if last_domain_bill.plan == plan => {
				let hours = min(
					720,
					((stop_time - last_domain_bill.start_time.0).num_seconds()
						as f64 / 3600f64)
						.ceil() as u64,
				);
				let price_in_dollars = if hours >= 720 {
					monthly_price
				} else {
					hours as f64 * (monthly_price / 720f64)
				};
				let price_in_cents = (price_in_dollars * 100.0).round() as u64;

				last_domain_bill.stop_time =
					domains_usage.stop_time.map(DateTime);
				last_domain_bill.hours = hours;
				last_domain_bill.amount = price_in_cents;
			}
			_ => {
				domains_bill.push(DomainUsage {
					plan,
					hours: hours as u64,
					amount: price_in_cents,
					start_time: DateTime(start_time),
					stop_time: domains_usage.stop_time.map(DateTime),
					monthly_charge: monthly_price as u64 * 100,
				});
			}
		}
	}

	Ok(domains_bill)
}

pub async fn calculate_secrets_bill_for_workspace_till(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	month_start_date: &chrono::DateTime<Utc>,
	till_date: &chrono::DateTime<Utc>,
) -> Result<Vec<SecretUsage>, Error> {
	let secrets_usages = db::get_all_secrets_usages(
		&mut *connection,
		workspace_id,
		&DateTime::from(*month_start_date),
		till_date,
	)
	.await?;

	let mut secrets_bill: Vec<SecretUsage> = Vec::new();
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

		let price_in_dollars = if hours >= 720 {
			monthly_price
		} else {
			(hours as f64 / 720f64) * monthly_price
		};
		let price_in_cents = (price_in_dollars * 100.0).round() as u64;

		let secrets = secrets_usage.secret_count as f64;
		let secrets = (secrets / 100f64).ceil() as u64 * 100;
		let plan = if secrets_usage.secret_count <= 3 {
			"0-3 Secrets".to_string()
		} else {
			format!("{}-{} Secrets", max(11, secrets - 100), secrets)
		};

		match secrets_bill.last_mut() {
			Some(last_secret_bill) if last_secret_bill.plan == plan => {
				let hours = min(
					720,
					((stop_time - last_secret_bill.start_time.0).num_seconds()
						as f64 / 3600f64)
						.ceil() as u64,
				);

				let price_in_dollars = if hours >= 720 {
					monthly_price
				} else {
					(hours as f64 / 720f64) * monthly_price
				};
				let price_in_cents = (price_in_dollars * 100.0).round() as u64;

				last_secret_bill.stop_time =
					secrets_usage.stop_time.map(DateTime);
				last_secret_bill.hours = hours;
				last_secret_bill.amount = price_in_cents;
			}
			_ => {
				secrets_bill.push(SecretUsage {
					plan,
					hours: hours as u64,
					amount: price_in_cents,
					start_time: DateTime(start_time),
					stop_time: secrets_usage.stop_time.map(DateTime),
					monthly_charge: monthly_price as u64 * 100,
				});
			}
		}
	}

	Ok(secrets_bill)
}

pub async fn add_card_details(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	config: &Settings,
) -> Result<PaymentIntent, Error> {
	let workspace = db::get_workspace_info(connection, workspace_id)
		.await?
		.status(500)?;

	let address_id = if let Some(address_id) = workspace.address_id {
		address_id
	} else {
		return Err(Error::empty()
			.status(400)
			.body(error!(ADDRESS_REQUIRED).to_string()));
	};

	let user = db::get_user_by_user_id(connection, &workspace.super_admin_id)
		.await?
		.status(500)?;

	let email_str = user
		.recovery_email_local
		.as_ref()
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	let domain = db::get_personal_domain_by_id(
		connection,
		user.recovery_email_domain_id
			.as_ref()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?,
	)
	.await?
	.status(500)?;

	let email = format!("{}@{}", email_str, domain.name);

	let email_spam_result = reqwest::Client::new()
		.get(format!(
			"{}/{}/{}",
			config.ip_quality.host, config.ip_quality.token, email
		))
		.send()
		.await
		.map_err(|error| {
			log::error!("IPQS api call error: {}", error);
			Error::from(error)
		})?
		.json::<IpQualityScore>()
		.await
		.map_err(|error| {
			log::error!("Error parsing IPQS response: {}", error);
			Error::from(error)
		})?;

	let amount_in_cents = match email_spam_result.fraud_score {
		(0..=50) => 200u64,  // $2 in cents
		(51..=80) => 500u64, // $5 in cents
		(81..=90) => 800u64, // $8 in cents
		_ => 1200u64,        // $12 in cents
	};

	let description = "Patr charge: Card verification charges";

	let (currency, amount) = if db::get_billing_address(connection, &address_id)
		.await?
		.status(500)?
		.country == *"IN"
	{
		(Currency::INR, amount_in_cents * 80)
	} else {
		(Currency::USD, amount_in_cents)
	};

	let client = Client::new(&config.stripe.secret_key);

	let payment_intent = PaymentIntent::create(&client, {
		let mut intent = CreatePaymentIntent::new(amount as i64, currency);

		intent.confirm = Some(false);
		intent.description = Some(description);
		intent.customer =
			Some(CustomerId::from_str(&workspace.stripe_customer_id)?);
		// Since we are not doing SetupIntent anymore, Follow this url of the
		// stripe documentation to know why are the below configuration the way they are done https://stripe.com/docs/api/payment_methods/attach

		// Since we are not doing setup intent, we are using the
		// setup_future_usage and automatic_payment_methods below, this will
		// make sure the payment_method is attached to a customer
		intent.setup_future_usage =
			Some(PaymentIntentSetupFutureUsage::OffSession);
		intent.payment_method_types = Some(vec!["card".to_string()]);

		intent
	})
	.await
	.map_err(|error| {
		log::error!("Error from stripe: {}", error);
		Error::from(error)
	})?;

	Ok(payment_intent)
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
			card: serde_json::from_str(&serde_json::to_string(
				&card_details.card.status(500)?,
			)?)?,
			created: DateTime::from(
				Utc.timestamp_opt(card_details.created, 0).unwrap(),
			),
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
) -> Result<WorkspaceBillBreakdown, Error> {
	let deployment_usage = calculate_deployment_bill_for_workspace_till(
		&mut *connection,
		workspace_id,
		month_start_date,
		till_date,
	)
	.await?;

	let volume_usage = calculate_volumes_bill_for_workspace_till(
		&mut *connection,
		workspace_id,
		month_start_date,
		till_date,
	)
	.await?;

	let database_usage = calculate_database_bill_for_workspace_till(
		&mut *connection,
		workspace_id,
		month_start_date,
		till_date,
	)
	.await?;

	let patr_database_usage = calculate_patr_database_bill_for_workspace_till(
		connection,
		workspace_id,
		month_start_date,
		till_date,
	)
	.await?;

	let static_site_usage = calculate_static_sites_bill_for_workspace_till(
		&mut *connection,
		workspace_id,
		month_start_date,
		till_date,
	)
	.await?;

	let managed_url_usage = calculate_managed_urls_bill_for_workspace_till(
		&mut *connection,
		workspace_id,
		month_start_date,
		till_date,
	)
	.await?;

	let docker_repository_usage =
		calculate_docker_repository_bill_for_workspace_till(
			&mut *connection,
			workspace_id,
			month_start_date,
			till_date,
		)
		.await?;

	let domain_usage = calculate_domains_bill_for_workspace_till(
		&mut *connection,
		workspace_id,
		month_start_date,
		till_date,
	)
	.await?;

	let secret_usage = calculate_secrets_bill_for_workspace_till(
		&mut *connection,
		workspace_id,
		month_start_date,
		till_date,
	)
	.await?;

	let deployment_charge = deployment_usage
		.iter()
		.map(|bill| {
			bill.deployment_bill
				.iter()
				.map(|item| item.amount)
				.sum::<u64>()
		})
		.sum();

	let volume_charge = volume_usage.iter().map(|bill| bill.amount).sum();
	let database_charge = database_usage.iter().map(|bill| bill.amount).sum();
	let patr_database_charge =
		patr_database_usage.iter().map(|bill| bill.amount).sum();
	let static_site_charge =
		static_site_usage.iter().map(|bill| bill.amount).sum();
	let managed_url_charge =
		managed_url_usage.iter().map(|bill| bill.amount).sum();
	let docker_repository_charge =
		docker_repository_usage.iter().map(|bill| bill.amount).sum();
	let domain_charge = domain_usage.iter().map(|bill| bill.amount).sum();
	let secret_charge = secret_usage.iter().map(|bill| bill.amount).sum();

	let total_charge = deployment_charge +
		database_charge +
		static_site_charge +
		managed_url_charge +
		docker_repository_charge +
		domain_charge +
		secret_charge;

	let month = month_start_date.month();
	let year = month_start_date.year() as u32;

	let total_resource_usage_bill = WorkspaceBillBreakdown {
		month,
		year,
		total_charge,
		deployment_charge,
		deployment_usage,
		volume_charge,
		volume_usage,
		database_charge,
		database_usage,
		static_site_charge,
		static_site_usage,
		managed_url_charge,
		managed_url_usage,
		docker_repository_charge,
		docker_repository_usage,
		domain_charge,
		domain_usage,
		secret_charge,
		secret_usage,
		patr_database_charge,
		patr_database_usage,
	};

	if total_charge > 0 && cfg!(debug_assertions) {
		log::trace!(
			"Total bill for workspace `{}`: {:?}",
			workspace_id,
			total_resource_usage_bill
		);
	}

	Ok(total_resource_usage_bill)
}

pub async fn get_total_resource_usage(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	month_start_date: &chrono::DateTime<Utc>,
	till_date: &chrono::DateTime<Utc>,
	year: u32,
	month: u32,
	request_id: &Uuid,
) -> Result<WorkspaceBillBreakdown, Error> {
	log::trace!(
		"request_id: {} getting bill for all deployments",
		request_id
	);
	let deployment_usage = calculate_deployment_bill_for_workspace_till(
		connection,
		workspace_id,
		month_start_date,
		till_date,
	)
	.await?;
	let deployment_charge = deployment_usage
		.iter()
		.map(|bill| {
			bill.deployment_bill
				.iter()
				.map(|item| item.amount)
				.sum::<u64>()
		})
		.sum();

	log::trace!("request_id: {} getting bill for all volumes", request_id);

	let volume_usage = calculate_volumes_bill_for_workspace_till(
		connection,
		workspace_id,
		month_start_date,
		till_date,
	)
	.await?;

	let volume_charge =
		volume_usage.iter().map(|bill| bill.amount).sum::<u64>();

	log::trace!(
		"request_id: {} getting bill for all managed_databases",
		request_id
	);

	let database_usage = calculate_database_bill_for_workspace_till(
		connection,
		workspace_id,
		month_start_date,
		till_date,
	)
	.await?;
	let database_charge = database_usage.iter().map(|bill| bill.amount).sum();

	log::trace!(
		"request_id: {} getting bill for all patr databases",
		request_id
	);
	let patr_database_usage = calculate_patr_database_bill_for_workspace_till(
		connection,
		workspace_id,
		month_start_date,
		till_date,
	)
	.await?;
	let patr_database_charge =
		patr_database_usage.iter().map(|bill| bill.amount).sum();

	log::trace!(
		"request_id: {} getting bill for all static_site",
		request_id
	);
	let static_site_usage = calculate_static_sites_bill_for_workspace_till(
		connection,
		workspace_id,
		month_start_date,
		till_date,
	)
	.await?;
	let static_site_charge =
		static_site_usage.iter().map(|bill| bill.amount).sum();

	log::trace!(
		"request_id: {} getting bill for all managed_urls",
		request_id
	);
	let managed_url_usage = calculate_managed_urls_bill_for_workspace_till(
		connection,
		workspace_id,
		month_start_date,
		till_date,
	)
	.await?;
	let managed_url_charge =
		managed_url_usage.iter().map(|bill| bill.amount).sum();

	log::trace!(
		"request_id: {} getting bill for all docker repos",
		request_id
	);
	let docker_repository_usage =
		calculate_docker_repository_bill_for_workspace_till(
			connection,
			workspace_id,
			month_start_date,
			till_date,
		)
		.await?;
	let docker_repository_charge =
		docker_repository_usage.iter().map(|bill| bill.amount).sum();

	log::trace!("request_id: {} getting bill for all domains", request_id);
	let domain_usage = calculate_domains_bill_for_workspace_till(
		connection,
		workspace_id,
		month_start_date,
		till_date,
	)
	.await?;
	let domain_charge = domain_usage.iter().map(|bill| bill.amount).sum();

	log::trace!("request_id: {} getting bill for all secrest", request_id);
	let secret_usage = calculate_secrets_bill_for_workspace_till(
		connection,
		workspace_id,
		month_start_date,
		till_date,
	)
	.await?;
	let secret_charge = secret_usage.iter().map(|bill| bill.amount).sum();

	let total_charge = deployment_charge +
		database_charge +
		static_site_charge +
		managed_url_charge +
		domain_charge +
		secret_charge +
		docker_repository_charge;

	log::trace!(
		"request_id: {} got total bill of : {}",
		request_id,
		total_charge
	);

	let bill = WorkspaceBillBreakdown {
		year,
		month,
		total_charge,
		deployment_charge,
		deployment_usage,
		volume_charge,
		volume_usage,
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
		patr_database_charge,
		patr_database_usage,
	};
	Ok(bill)
}
