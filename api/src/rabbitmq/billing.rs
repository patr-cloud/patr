use std::{
	ops::{Add, Sub},
	time::Duration,
};

use api_models::utils::{DateTime, Uuid};
use chrono::{TimeZone, Utc};
use eve_rs::AsError;
use reqwest::Client;
use tokio::time;

use crate::{
	db::{self, PaymentStatus, TransactionType},
	error,
	models::{
		rabbitmq::WorkspaceRequestData,
		PaymentIntent,
		PaymentIntentObject,
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
			let workspaces = db::get_all_workspaces(connection).await?;

			for workspace in workspaces {
				log::trace!(
					"request_id: {} - Processing workspace: {}",
					request_id,
					workspace.id
				);

				super::queue_generate_invoice_for_workspace(
					config, workspace, month, year,
				)
				.await?;
			}
			Ok(())
		}
		WorkspaceRequestData::GenerateInvoice {
			month,
			year,
			workspace,
			request_id,
		} => {
			let month_start_date = Utc.ymd(year, month, 1).and_hms(0, 0, 0);
			let next_month_start_date = Utc
				.ymd(
					if month == 12 { year + 1 } else { year },
					if month == 12 { 1 } else { month + 1 },
					1,
				)
				.and_hms(0, 0, 0)
				.sub(chrono::Duration::nanoseconds(1));

			// Step 1: Calculate bill for this entire cycle
			let total_bill = service::calculate_total_bill_for_workspace_till(
				&mut *connection,
				&workspace.id,
				&month_start_date,
				&next_month_start_date.into(),
			)
			.await?;

			// Step 2: Create payment intent with the given bill
			let password: Option<String> = None;

			let payment_intent_object = Client::new()
				.post("https://api.stripe.com/v1/payment_intents")
				.basic_auth(&config.stripe.secret_key, password)
				.form(&PaymentIntent {
					amount: total_bill,
					currency: "usd".to_string(),
					description: format!(
						"Patr charge: Bill for {} {}",
						match month {
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
						},
						year
					),
					customer: config.stripe.customer_id.clone(),
				})
				.send()
				.await?
				.json::<PaymentIntentObject>()
				.await?;

			// create transactions for all types of resources
			let transaction_id =
				db::generate_new_transaction_id(connection).await?;
			db::create_transactions(
				connection,
				&workspace.id,
				&transaction_id,
				month as i32,
				total_bill as i64,
				Some(&payment_intent_object.id),
				&DateTime::from(
					next_month_start_date.add(chrono::Duration::nanoseconds(1)),
				), // 1st of next month,
				&TransactionType::Bill,
				&PaymentStatus::Success,
			)
			.await?;

			super::queue_confirm_payment_intent(
				config,
				payment_intent_object.id,
				workspace.id.clone(),
			)
			.await?;

			send_invoice_email_to_workspace(
				connection,
				&workspace.super_admin_id,
				&workspace.id,
				&workspace.name,
				total_bill,
				&month_start_date,
				&next_month_start_date.into(),
			)
			.await?;

			Ok(())
		}
		WorkspaceRequestData::ConfirmPaymentIntent {
			payment_intent_id,
			workspace_id,
			request_id,
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
							last_transaction.date.into();
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

			let client = Client::new();

			let password: Option<String> = None;

			let transaction_id =
				db::generate_new_transaction_id(connection).await?;

			let payment_status = client
				.post(format!(
					"https://api.stripe.com/v1/payment_intents/{}",
					payment_intent_id
				))
				.basic_auth(&config.stripe.secret_key, password)
				.send()
				.await?
				.status();

			if !payment_status.is_success() {
				db::create_transactions(
					connection,
					&workspace_id,
					&transaction_id,
					last_transaction.month,
					last_transaction.amount,
					Some(&payment_intent_id),
					&DateTime::from(Utc::now()),
					&TransactionType::Payment,
					&PaymentStatus::Failed,
				)
				.await?;

				return Error::as_result()
					.status(500)
					.body(error!(SERVER_ERROR).to_string())?;
			}

			db::create_transactions(
				connection,
				&workspace_id,
				&transaction_id,
				last_transaction.month,
				last_transaction.amount,
				Some(&payment_intent_id),
				&(Utc::now().into()),
				&TransactionType::Payment,
				&PaymentStatus::Success,
			)
			.await?;

			Ok(())
		}
	}
}

async fn send_invoice_email_to_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
	workspace_id: &Uuid,
	workspace_name: &str,
	total_bill: u64,
	start_date: &chrono::DateTime<Utc>,
	end_date: &chrono::DateTime<Utc>,
) -> Result<(), Error> {
	let deployment_usage = service::get_deployment_usage_and_bill(
		connection,
		workspace_id,
		start_date,
		end_date,
	)
	.await?;

	let static_site_usage_bill = service::get_static_site_usage_and_bill(
		connection,
		workspace_id,
		start_date,
		end_date,
	)
	.await?;

	let database_usage = service::get_database_usage_and_bill(
		connection,
		workspace_id,
		start_date,
		end_date,
	)
	.await?;

	let managed_url_usage_bill = service::get_managed_url_usage_and_bill(
		connection,
		workspace_id,
		start_date,
		end_date,
	)
	.await?;

	let secret_usage_bill = service::get_secret_usage_and_bill(
		connection,
		workspace_id,
		start_date,
		end_date,
	)
	.await?;

	let docker_repo_usage_bill = service::get_docker_repo_usage_and_bill(
		connection,
		workspace_id,
		start_date,
		end_date,
	)
	.await?;

	let domain_usage_bill = service::get_domain_usage_and_bill(
		connection,
		workspace_id,
		start_date,
		end_date,
	)
	.await?;

	service::send_invoice_email(
		connection,
		user_id,
		workspace_id,
		workspace_name,
		deployment_usage,
		static_site_usage_bill,
		database_usage,
		managed_url_usage_bill,
		secret_usage_bill,
		docker_repo_usage_bill,
		domain_usage_bill,
		total_bill as u64,
		start_date,
		end_date,
	)
	.await?;

	Ok(())
}
