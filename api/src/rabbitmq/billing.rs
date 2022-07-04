use std::{
	ops::{Add, Sub},
	time::Duration,
};

use api_models::{
	models::workspace::billing::{PaymentStatus, TransactionType},
	utils::{DateTime, True},
};
use chrono::{Datelike, TimeZone, Utc};
use eve_rs::AsError;
use reqwest::Client;
use tokio::time;

use crate::{
	db::{self, PaymentType},
	error,
	models::{
		billing::{PaymentIntent, PaymentIntentObject},
		rabbitmq::WorkspaceRequestData,
	},
	service,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn process_request(
	connection: &mut <Database as sqlx::Database>::Connection,
	request_data: WorkspaceRequestData,
	config: &Settings,
) -> Result<(), Error> {
	match request_data {
		WorkspaceRequestData::ProcessWorkspaces {
			month,
			year,
			request_id,
		} => {
			if Utc::now() < Utc.ymd(year, month, 1).and_hms(0, 0, 0) {
				// It's not yet time to process the workspace. Wait and try
				// again later
				time::sleep(Duration::from_millis(
					if cfg!(debug_assertions) { 1000 } else { 60_000 },
				))
				.await;
				return Error::as_result()
					.status(500)
					.body(error!(SERVER_ERROR).to_string())?;
			}
			let workspaces = db::get_all_workspaces(connection).await?;
			log::trace!(
				"request_id: {} - Processing workspace for {} {}",
				request_id,
				month,
				year
			);

			for workspace in workspaces {
				log::trace!(
					"request_id: {} - Processing workspace: {}",
					request_id,
					workspace.id
				);

				service::queue_generate_invoice_for_workspace(
					config, workspace, month, year,
				)
				.await?;
			}

			service::queue_process_payment(
				if month == 12 { 1 } else { month + 1 },
				if month == 12 { year + 1 } else { year },
				config,
			)
			.await?;

			Ok(())
		}
		WorkspaceRequestData::GenerateInvoice {
			month,
			year,
			workspace,
			request_id,
		} => {
			log::trace!(
				"request_id: {} - Generating invoice for {} {}",
				request_id,
				month,
				year
			);
			let month_start_date = Utc.ymd(year, month, 1).and_hms(0, 0, 0);
			let next_month_start_date = Utc
				.ymd(
					if month == 12 { year + 1 } else { year },
					if month == 12 { 1 } else { month + 1 },
					1,
				)
				.and_hms(0, 0, 0)
				.sub(chrono::Duration::nanoseconds(1));

			let month_string = match month {
				1 => "January",
				2 => "February",
				3 => "March",
				4 => "April",
				5 => "May",
				6 => "June",
				7 => "July",
				8 => "August",
				9 => "September",
				10 => "October",
				11 => "November",
				12 => "December",
				_ => "",
			};

			// Step 1: Calculate bill for this entire cycle
			let deployment_usages =
				service::calculate_deployment_bill_for_workspace_till(
					connection,
					&workspace.id,
					&month_start_date,
					&next_month_start_date,
				)
				.await?;

			let database_usages =
				service::calculate_database_bill_for_workspace_till(
					connection,
					&workspace.id,
					&month_start_date,
					&next_month_start_date,
				)
				.await?;

			let static_sites_usages =
				service::calculate_static_sites_bill_for_workspace_till(
					connection,
					&workspace.id,
					&month_start_date,
					&next_month_start_date,
				)
				.await?;

			let managed_url_usages =
				service::calculate_managed_urls_bill_for_workspace_till(
					connection,
					&workspace.id,
					&month_start_date,
					&next_month_start_date,
				)
				.await?;

			let docker_repository_usages =
				service::calculate_docker_repository_bill_for_workspace_till(
					connection,
					&workspace.id,
					&month_start_date,
					&next_month_start_date,
				)
				.await?;

			let domains_usages =
				service::calculate_domains_bill_for_workspace_till(
					connection,
					&workspace.id,
					&month_start_date,
					&next_month_start_date,
				)
				.await?;

			let secrets_usages =
				service::calculate_secrets_bill_for_workspace_till(
					connection,
					&workspace.id,
					&month_start_date,
					&next_month_start_date,
				)
				.await?;

			let total_bill = service::calculate_total_bill_for_workspace_till(
				connection,
				&workspace.id,
				&month_start_date,
				&next_month_start_date,
			)
			.await?;

			// Step 2: Create payment intent with the given bill
			let password: Option<String> = None;

			if let PaymentType::Card = workspace.payment_type {
				if total_bill <= 0.0 {
					// If the bill is zero, don't bother charging them
					return Ok(());
				}

				let address_id = workspace
					.clone()
					.address_id
					.status(500)
					.body(error!(SERVER_ERROR).to_string())?;

				let (currency, amount) =
					if db::get_billing_address(connection, &address_id)
						.await?
						.status(500)?
						.country == *"IN"
					{
						(
							"inr".to_string(),
							(total_bill * 100f64 * 80f64) as u64,
						)
					} else {
						("usd".to_string(), (total_bill * 100f64) as u64)
					};

				let payment_intent_object = Client::new()
					.post("https://api.stripe.com/v1/payment_intents")
					.basic_auth(&config.stripe.secret_key, password)
					.form(&PaymentIntent {
						amount,
						currency,
						confirm: True,
						off_session: true,
						description: format!(
							"Patr charge: Bill for {} {}",
							month_string, year
						),
						customer: workspace.stripe_customer_id.clone(),
						payment_method: workspace.default_payment_method_id,
						payment_method_types: "card".to_string(),
						setup_future_usage: None,
					})
					.send()
					.await?
					.json::<PaymentIntentObject>()
					.await?;

				// create transactions for all types of resources
				let transaction_id =
					db::generate_new_transaction_id(connection).await?;
				db::create_transaction(
					connection,
					&workspace.id,
					&transaction_id,
					month as i32,
					total_bill,
					Some(&payment_intent_object.id),
					&DateTime::from(
						next_month_start_date
							.add(chrono::Duration::nanoseconds(1)),
					), // 1st of next month,
					&TransactionType::Bill,
					&PaymentStatus::Success,
					None,
				)
				.await?;

				service::queue_confirm_payment_intent(
					config,
					payment_intent_object.id,
					&month_start_date,
					workspace.id.clone(),
				)
				.await?;
			} else {
				// create transactions for all types of resources
				let transaction_id =
					db::generate_new_transaction_id(connection).await?;
				db::create_transaction(
					connection,
					&workspace.id,
					&transaction_id,
					month as i32,
					total_bill,
					Some("enterprise-plan-bill"),
					&DateTime::from(
						next_month_start_date
							.add(chrono::Duration::nanoseconds(1)),
					), // 1st of next month,
					&TransactionType::Bill,
					&PaymentStatus::Success,
					None,
				)
				.await?;

				// Enterprise plan. Just assume a payment is made
				let transaction_id =
					db::generate_new_transaction_id(connection).await?;
				db::create_transaction(
					connection,
					&workspace.id,
					&transaction_id,
					month as i32,
					total_bill,
					Some("enterprise-plan-payment"),
					&DateTime::from(
						next_month_start_date
							.add(chrono::Duration::nanoseconds(1)),
					), // 1st of next month,
					&TransactionType::Payment,
					&PaymentStatus::Success,
					None,
				)
				.await?;
			}

			service::send_invoice_email(
				connection,
				&workspace.super_admin_id,
				workspace.name.clone(),
				deployment_usages,
				database_usages,
				static_sites_usages,
				managed_url_usages,
				docker_repository_usages,
				domains_usages,
				secrets_usages,
				total_bill,
				month_string.to_string(),
				year,
			)
			.await?;

			Ok(())
		}
		WorkspaceRequestData::ConfirmPaymentIntent {
			payment_intent_id,
			workspace_id,
			request_id,
			month_start_date,
		} => {
			let last_transaction = db::get_last_bill_for_workspace(
				&mut *connection,
				&workspace_id,
				payment_intent_id.clone(),
			)
			.await?;
			let last_transaction = if let Some(transaction) = last_transaction {
				transaction
			} else {
				// TODO report here
				return Ok(());
			};

			if last_transaction.payment_status == PaymentStatus::Success {
				log::warn!(
					"request_id: {} - Already paid for workspace: {}",
					request_id,
					workspace_id
				);
				return Ok(());
			} else if last_transaction.payment_status == PaymentStatus::Failed {
				// Check timestamp
				if Utc::now()
					.sub({
						let chrono_date: chrono::DateTime<Utc> =
							last_transaction.date.clone().into();
						chrono_date
					})
					.num_hours()
					.abs() > 24
				{
					// It's been more than 24 hours since the last transaction
					// attempt
				} else {
					// It's been less than 24 hours since the last transaction
					// attempt Wait for a while and requeue this task
					time::sleep(Duration::from_millis(60_000)).await;
					return Error::as_result()
						.status(500)
						.body(error!(SERVER_ERROR).to_string())?;
				}
			}
			// confirming payment intent and charging the user

			let month_string = match last_transaction.month {
				1 => "January",
				2 => "February",
				3 => "March",
				4 => "April",
				5 => "May",
				6 => "June",
				7 => "July",
				8 => "August",
				9 => "September",
				10 => "October",
				11 => "November",
				12 => "December",
				_ => "",
			};

			let year =
				chrono::DateTime::from(last_transaction.date.clone()).year();

			let client = Client::new();

			let password: Option<String> = None;

			let transaction_id =
				db::generate_new_transaction_id(connection).await?;

			let payment_status = client
				.post(format!(
					"https://api.stripe.com/v1/payment_intents/{}/confirm",
					payment_intent_id
				))
				.basic_auth(&config.stripe.secret_key, password)
				.send()
				.await?
				.status();

			if !payment_status.is_success() {
				service::send_payment_failed_reminder(
					connection,
					&workspace_id,
					last_transaction.amount,
					month_string,
					year,
				)
				.await?;

				db::create_transaction(
					connection,
					&workspace_id,
					&transaction_id,
					last_transaction.month,
					last_transaction.amount,
					Some(&payment_intent_id),
					&DateTime::from(Utc::now()),
					&TransactionType::Payment,
					&PaymentStatus::Failed,
					None,
				)
				.await?;

				// if it is more than 14 days since the bill has been out
				// try one last time and if that transaction fails
				// then shutdown all resources and change the workspace limit to
				// 0
				// TODO: send user reminder for non-payment

				if Utc::now().sub(month_start_date).num_days().abs() > 14 {
					service::shutdown_all_services(
						connection,
						&workspace_id,
						config,
					)
					.await?;

					service::send_delete_resource_notification(
						connection,
						&workspace_id,
						month_string,
						year,
					)
					.await?;
				}

				return Error::as_result()
					.status(500)
					.body(error!(SERVER_ERROR).to_string())?;
			}

			db::create_transaction(
				connection,
				&workspace_id,
				&transaction_id,
				last_transaction.month,
				last_transaction.amount,
				Some(&payment_intent_id),
				&(Utc::now().into()),
				&TransactionType::Payment,
				&PaymentStatus::Success,
				None,
			)
			.await?;

			Ok(())
		}
	}
}
