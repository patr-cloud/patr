use std::{
	ops::{Add, Sub},
	str::FromStr,
};

use api_models::{
	models::workspace::billing::{PaymentStatus, TransactionType},
	utils::DateTime,
};
use chrono::{Duration, TimeZone, Utc};
use eve_rs::AsError;
use stripe::{
	Client,
	CreatePaymentIntent,
	Currency,
	CustomerId,
	OffSessionOther,
	PaymentIntent,
	PaymentIntentConfirmationMethod,
	PaymentIntentOffSession,
	PaymentIntentSetupFutureUsage,
	PaymentIntentStatus,
	PaymentMethodId,
	RequestStrategy,
};
use tokio::time;

use crate::{
	db::{self, PaymentType},
	models::rabbitmq::WorkspaceRequestData,
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
				return Err(Error::empty());
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
			let total_resource_usage_bill =
				service::calculate_total_bill_for_workspace_till(
					connection,
					&workspace.id,
					&month_start_date,
					&next_month_start_date,
				)
				.await?;

			// create transactions for the bill
			let transaction_id =
				db::generate_new_transaction_id(connection).await?;
			db::create_transaction(
				connection,
				&workspace.id,
				&transaction_id,
				month as i32,
				total_resource_usage_bill.total_cost.parse()?,
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

			let credit_amount = total_resource_usage_bill
				.total_cost
				.parse::<f64>()
				.unwrap_or_default() - // Can be handled better instead of unwrap_or_default
				payable_bill;

			let credit_remaining = if payable_bill >= 0.00 {
				0.00
			} else {
				payable_bill * -1.00 // To make it positive
			};

			service::queue_attempt_to_charge_workspace(
				&workspace,
				&Utc::now(),
				&total_resource_usage_bill,
				payable_bill,
				credit_amount,
				credit_remaining,
				month,
				year,
				config,
			)
			.await?;

			Ok(())
		}
		WorkspaceRequestData::AttemptToChargeWorkspace {
			workspace,
			process_after: DateTime(process_after),
			total_resource_usage_bill,
			amount_due,
			credit_amount,
			credit_remaining,
			month,
			year,
			request_id,
		} => {
			log::trace!("request_id: {} attempting to charge user", request_id);

			if Utc::now() < process_after {
				// process_after is in the future. Wait for a while and requeue
				time::sleep(time::Duration::from_millis(
					if cfg!(debug_assertions) { 1000 } else { 60_000 },
				))
				.await;
				return Err(Error::empty());
			}

			let month_end_date = Utc
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

			if amount_due <= 0.0 {
				service::send_payment_success_notification(
					connection,
					workspace.super_admin_id.clone(),
					workspace.name.clone(),
					total_resource_usage_bill.deployment_usages.to_owned(),
					total_resource_usage_bill.database_usages.to_owned(),
					total_resource_usage_bill.static_sites_usages.to_owned(),
					total_resource_usage_bill.managed_url_usages.to_owned(),
					total_resource_usage_bill
						.docker_repository_usages
						.to_owned(),
					total_resource_usage_bill.domains_usages.to_owned(),
					total_resource_usage_bill.secrets_usages.to_owned(),
					month_string.to_string(),
					month,
					year,
					format!("{:.2}", amount_due * -1.00), /* To make it
					                                       * positive */
					format!(
						"{:.2}",
						if credit_amount > 0.00 {
							credit_amount
						} else {
							0.00
						}
					),
					"0.00".to_string(),
					format!("{:.2}", credit_remaining),
					format!("{:.2}", amount_due * -1.0),
				)
				.await?;
				return Ok(());
			}

			// Step 2: Create payment intent with the given bill
			if let PaymentType::Card = workspace.payment_type {
				if let (Some(address_id), Some(payment_method_id)) = (
					&workspace.address_id,
					&workspace.default_payment_method_id,
				) {
					let (currency, stripe_amount) =
						if db::get_billing_address(connection, address_id)
							.await?
							.status(500)?
							.country == *"IN"
						{
							(
								Currency::INR,
								(amount_due * 100f64 * 80f64) as i64,
							)
						} else {
							(Currency::USD, (amount_due * 100f64) as i64)
						};

					let client = Client::new(&config.stripe.secret_key)
						.with_strategy(RequestStrategy::Idempotent(format!(
							"{}-{}-{}",
							workspace.id, month, year
						)));
					let payment_description = Some(format!(
						"Patr charge: Bill for {} {}",
						month_string, year
					));
					let payment_intent = PaymentIntent::create(&client, {
						let mut intent =
							CreatePaymentIntent::new(stripe_amount, currency);

						intent.confirm = Some(true);
						intent.confirmation_method =
							Some(PaymentIntentConfirmationMethod::Automatic);
						intent.off_session =
							Some(PaymentIntentOffSession::Other(
								OffSessionOther::OneOff,
							));
						intent.description = payment_description.as_deref();
						intent.customer = Some(CustomerId::from_str(
							&workspace.stripe_customer_id,
						)?);
						intent.payment_method =
							Some(PaymentMethodId::from_str(payment_method_id)?);
						intent.payment_method_types =
							Some(vec!["card".to_string()]);
						intent.setup_future_usage =
							Some(PaymentIntentSetupFutureUsage::OffSession);

						intent
					})
					.await?;

					let payment_intent = PaymentIntent::confirm(
						&client,
						&payment_intent.id,
						Default::default(),
					)
					.await?;

					let transaction_id =
						db::generate_new_transaction_id(connection).await?;

					// Payment will only has successful or failed state
					// according to stripe docs. For more information.
					// check - https://stripe.com/docs/payments/payment-intents/verifying-status
					if let PaymentIntentStatus::Succeeded =
						payment_intent.status
					{
						db::create_transaction(
							connection,
							&workspace.id,
							&transaction_id,
							month as i32,
							amount_due,
							Some(&payment_intent.id),
							&Utc::now(),
							&TransactionType::Payment,
							&PaymentStatus::Success,
							None,
						)
						.await?;

						service::send_payment_success_notification(
							connection,
							workspace.super_admin_id.clone(),
							workspace.name.clone(),
							total_resource_usage_bill.deployment_usages,
							total_resource_usage_bill.database_usages,
							total_resource_usage_bill.static_sites_usages,
							total_resource_usage_bill.managed_url_usages,
							total_resource_usage_bill.docker_repository_usages,
							total_resource_usage_bill.domains_usages,
							total_resource_usage_bill.secrets_usages,
							month_string.to_string(),
							month,
							year,
							format!("{:.2}", amount_due),
							format!(
								"{:.2}",
								if credit_amount > 0.00 {
									credit_amount
								} else {
									0.00
								}
							),
							format!("{:.2}", amount_due),
							format!("{:.2}", credit_remaining),
							format!("{:.2}", amount_due),
						)
						.await?;

						Ok(())
					} else {
						db::create_transaction(
							connection,
							&workspace.id,
							&transaction_id,
							month as i32,
							amount_due,
							Some(&payment_intent.id),
							&Utc::now(),
							&TransactionType::Payment,
							&PaymentStatus::Failed,
							None,
						)
						.await?;

						// notify customer that the payment has failed
						if (Utc::now() - month_end_date) > Duration::days(15) {
							// It's been more than 15 days since the bill was
							// generated and the payment has still not been
							// made. Lock the workspace and delete all resources

							// delete all resources
							service::delete_all_resources_in_workspace(
								connection,
								&workspace.id,
								config,
								&request_id,
							)
							.await?;

							// Reset resource limit to zero
							db::set_resource_limit_for_workspace(
								connection,
								&workspace.id,
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
							service::send_unpaid_resources_deleted_email(
								connection,
								workspace.super_admin_id.clone(),
								workspace.name.clone(),
								month_string.to_string(),
								month,
								year,
								amount_due,
							)
							.await?;

							Ok(())
						} else {
							service::send_payment_failed_notification(
								connection,
								workspace.super_admin_id.clone(),
								workspace.name.clone(),
								&total_resource_usage_bill.deployment_usages,
								&total_resource_usage_bill.database_usages,
								&total_resource_usage_bill.static_sites_usages,
								&total_resource_usage_bill.managed_url_usages,
								&total_resource_usage_bill
									.docker_repository_usages,
								&total_resource_usage_bill.domains_usages,
								&total_resource_usage_bill.secrets_usages,
								month_string.to_string(),
								month,
								year,
								amount_due,
							)
							.await?;

							// Requeue and try again in 24 hours
							service::queue_attempt_to_charge_workspace(
								&workspace,
								&Utc::now().add(Duration::days(1)),
								&total_resource_usage_bill,
								amount_due,
								credit_amount,
								credit_remaining,
								month,
								year,
								config,
							)
							.await?;

							Ok(())
						}
					}
				} else {
					// notify about the missing address and/or payment method
					// and reinitiate the payment process.
					// Send a reminder to user to pay for the resource that they
					// have consumed, and that they need to add a card for their
					// payment to complete
					log::trace!(
						concat!(
							"Payment method ID not found for workspace: {}, ",
							"calling reminder queue to send the",
							" mail reminder to pay for their usage"
						),
						workspace.id
					);

					if (Utc::now() - month_end_date) > Duration::days(15) {
						// It's been more than 15 days since the bill was
						// generated and the payment has still not been made.
						// Lock the workspace and delete all resources

						// delete all resources
						service::delete_all_resources_in_workspace(
							connection,
							&workspace.id,
							config,
							&request_id,
						)
						.await?;

						// Reset resource limit to zero
						db::set_resource_limit_for_workspace(
							connection,
							&workspace.id,
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
						service::send_unpaid_resources_deleted_email(
							connection,
							workspace.super_admin_id.clone(),
							workspace.name.clone(),
							month_string.to_string(),
							month,
							year,
							amount_due,
						)
						.await?;
					} else {
						let deadline_day = 15;
						let next_month =
							if month == 12 { 1 } else { month + 1 };

						let next_month_name_str = match next_month {
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

						let deadline = format!(
							"{deadline_day}{} {next_month_name_str}",
							match deadline_day {
								1 => "st",
								2 => "nd",
								3 => "rd",
								_ => "th",
							}
						);

						service::send_bill_not_paid_reminder_email(
							connection,
							workspace.super_admin_id.clone(),
							workspace.name.clone(),
							month_string.to_string(),
							month,
							year,
							amount_due,
							deadline,
						)
						.await?;

						// Requeue and try again in 24 hours
						service::queue_attempt_to_charge_workspace(
							&workspace,
							&Utc::now().add(Duration::days(1)),
							&total_resource_usage_bill,
							amount_due,
							credit_amount,
							credit_remaining,
							month,
							year,
							config,
						)
						.await?;
					}

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
					total_resource_usage_bill.total_cost.parse()?,
					Some("enterprise-plan-payment"),
					// 1st of next month,
					&month_end_date.add(Duration::nanoseconds(1)),
					&TransactionType::Payment,
					&PaymentStatus::Success,
					None,
				)
				.await?;

				Ok(())
			}
		}
	}
}
