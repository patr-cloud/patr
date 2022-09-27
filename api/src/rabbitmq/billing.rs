use std::ops::{Add, Sub};

use api_models::{
	models::workspace::billing::{PaymentStatus, TransactionType},
	utils::{True, Uuid},
};
use chrono::{Datelike, Duration, Month, TimeZone, Utc};
use eve_rs::AsError;
use num_traits::FromPrimitive;
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
				time::sleep(time::Duration::from_millis(
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

				// generate invoice for prev month
				let (month, year) = if month == 1 {
					(12, year - 1)
				} else {
					(month - 1, year)
				};
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
				"request_id: {} - Generating invoice for  workspace_id: {} for month: {} and year: {}",
				request_id,
				workspace.id,
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
				.sub(Duration::nanoseconds(1));

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
			let total_bill = service::calculate_total_bill_for_workspace_till(
				connection,
				&workspace.id,
				&month_start_date,
				&next_month_start_date,
			)
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
				None,
				// 1st of next month,
				&next_month_start_date.add(Duration::nanoseconds(1)),
				&TransactionType::Bill,
				&PaymentStatus::Success,
				Some(&format!("Bill for {} {}", month_string, year)),
			)
			.await?;

			let payable_bill = db::get_total_amount_to_pay_for_workspace(
				connection,
				&workspace.id,
			)
			.await?;

			service::queue_attempt_payment_intent(
				&workspace,
				total_bill,
				payable_bill,
				month_string,
				month,
				&next_month_start_date,
				year,
				config,
			)
			.await?;

			Ok(())
		}

		WorkspaceRequestData::AttemptPaymentIntent {
			workspace,
			total_bill,
			payable_bill,
			month_string,
			month,
			next_month_start_date,
			year,
			request_id,
		} => {
			// removed function "get_last_bill_for_workspace()" as it is not
			// being used after splitting generate invoice rmq job and clubbing
			// attempt_payment, confirm_payment and reminder jobs

			log::trace!(
				"request_id: {} attempting to make payment",
				request_id
			);

			if payable_bill <= 0.0 {
				// If the bill is zero, don't bother charging them
				return Ok(());
			}

			// Step 2: Create payment intent with the given bill
			let password: Option<String> = None;
			if let PaymentType::Card = workspace.payment_type {
				if let Some(address_id) = &workspace.address_id {
					let (currency, amount) =
						if db::get_billing_address(connection, address_id)
							.await?
							.status(500)?
							.country == *"IN"
						{
							(
								"inr".to_string(),
								(payable_bill * 100f64 * 80f64) as u64,
							)
						} else {
							("usd".to_string(), (payable_bill * 100f64) as u64)
						};

					let payment_intent_object = Client::new()
						.post("https://api.stripe.com/v1/payment_intents")
						.basic_auth(&config.stripe.secret_key, password.clone())
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

					let transaction_id =
						db::generate_new_transaction_id(connection).await?;

					let payment_status = Client::new()
						.post(format!(
							"https://api.stripe.com/v1/payment_intents/{}/confirm",
							payment_intent_object.id
						))
						.basic_auth(&config.stripe.secret_key, password)
						.send()
						.await?
						.status();

					// Payment will only has successful or failed state
					// according to stripe docs. For more information.
					// check - https://stripe.com/docs/payments/payment-intents/verifying-status
					if !payment_status.is_success() {
						db::create_transaction(
							connection,
							&workspace.id,
							&transaction_id,
							month as i32,
							payable_bill,
							Some(&payment_intent_object.id),
							&Utc::now(),
							&TransactionType::Payment,
							&PaymentStatus::Failed,
							None,
						)
						.await?;

						// TODO - notify customer when the payment is failed

						service::send_payment_failed_notification(
							connection,
							workspace.super_admin_id,
							workspace.name,
							month_string,
							year,
							payable_bill,
						)
						.await?;

						// Call reminder function which will take care of
						// reminder mail and deleting resource when unpaid

						patr_resource_usage_reminder(
							connection,
							&workspace.id,
							config,
							&request_id,
						)
						.await?;
						Ok(())
					} else {
						db::create_transaction(
							connection,
							&workspace.id,
							&transaction_id,
							month as i32,
							payable_bill,
							Some(&payment_intent_object.id),
							&Utc::now(),
							&TransactionType::Payment,
							&PaymentStatus::Success,
							None,
						)
						.await?;
						Ok(())
					}
				} else {
					// TODO: notify about the missing address id and reinitiate
					// the payment process once added. For now, using the
					// payment_indent_id as NULL

					// Setting a reminder to user to pay for the resource they
					// used as they have not added there card and there is
					// generated
					log::trace!("Addresss not found for workspace: {}, calling reminder queue to send the mail reminder to pay for there usage", workspace.id);
					patr_resource_usage_reminder(
						connection,
						&workspace.id,
						config,
						&request_id,
					)
					.await?;
					Ok(())
				}
			} else {
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
					// 1st of next month,
					&next_month_start_date.add(Duration::nanoseconds(1)),
					&TransactionType::Payment,
					&PaymentStatus::Success,
					None,
				)
				.await?;
				Ok(())
			}

			// TODO: for now disabled the invoice email,
			// but we need to enable this in next migration

			// service::send_invoice_email(
			// 	connection,
			// 	&workspace.super_admin_id,
			// 	workspace.name.clone(),
			// 	deployment_usages,
			// 	database_usages,
			// 	static_sites_usages,
			// 	managed_url_usages,
			// 	docker_repository_usages,
			// 	domains_usages,
			// 	secrets_usages,
			// 	total_bill,
			// 	month_string.to_string(),
			// 	year,
			// )
			// .await?;
		}
	}
}

pub async fn patr_resource_usage_reminder(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	// reminder mail
	let now = Utc::now();
	// Get the day of the month as invoice is generated at the end
	// of the month, with that logic current month should start from
	// 1st of every month
	let current_month_day = now.day();
	let workspace = db::get_workspace_info(connection, workspace_id).await?;

	if let Some(workspace) = workspace {
		let total_amount =
			db::get_total_amount_to_pay_for_workspace(connection, workspace_id)
				.await?;
		if total_amount > 0.0 {
			// Get previous month
			let month =
				Month::from_u32(now.date().month()).unwrap().pred().name();

			// Checking if current month is january then the year
			// should be last year else the current year
			// e.g if the bill is generated in year 2023 the bill
			// would be for december 2022
			let year = if now.date().month() == 1 {
				now.date().year() - 1
			} else {
				now.date().year()
			};
			if current_month_day < 15 {
				// send reminder mail for payment daily for 15 days
				service::send_bill_not_paid_reminder_email(
					connection,
					workspace.super_admin_id,
					workspace.name,
					month.to_owned(),
					year,
					total_amount,
				)
				.await?;

				Ok(())
			} else {
				// delete all resources
				service::delete_all_resources_in_workspace(
					connection,
					workspace_id,
					&workspace.super_admin_id,
					config,
					request_id,
				)
				.await?;

				// Reset resource limit to zero
				db::set_resource_limit_for_workspace(
					connection,
					workspace_id,
					0,
					0,
					0,
					0,
					0,
					0,
					0,
				)
				.await?;

				// send an mail
				service::send_delete_unpaid_resource_email(
					connection,
					workspace.super_admin_id.clone(),
					workspace.name.clone(),
					month.to_string(),
					year,
					total_amount,
				)
				.await?;

				Ok(())
			}
		} else {
			log::trace!("The amount is not payable as of now, continueing..");
			Ok(())
		}
	} else {
		log::trace!(
			"Workspace: {} not found. Not possible. Check logic",
			workspace_id
		);
		Ok(())
	}
}
