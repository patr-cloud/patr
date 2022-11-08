use std::{
	ops::{Add, Sub},
	str::FromStr,
};

use api_models::{
	models::workspace::billing::{Address, PaymentStatus, TransactionType},
	utils::{DateTime, PriceAmount, Uuid},
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
	models::rabbitmq::BillingData,
	service,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn process_request(
	connection: &mut <Database as sqlx::Database>::Connection,
	request_data: BillingData,
	config: &Settings,
) -> Result<(), Error> {
	match request_data {
		BillingData::ProcessWorkspaces {
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
		BillingData::GenerateInvoice {
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
			let total_bill_breakdown =
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
				total_bill_breakdown.total_charge.0,
				None,
				// 1st of next month,
				&next_month_start_date.add(Duration::nanoseconds(1)),
				&TransactionType::Bill,
				&PaymentStatus::Success,
				Some(&format!("Bill for {} {}", month_string, year)),
			)
			.await?;

			service::queue_attempt_to_charge_workspace(
				&workspace, month, year, config,
			)
			.await?;

			Ok(())
		}
		BillingData::SendInvoiceForWorkspace {
			workspace,
			month,
			year,
			request_id,
		} => {
			log::trace!("request_id: {} attempting to charge user", request_id);

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

			let month_start_date = Utc.ymd(year, month, 1).and_hms(0, 0, 0);
			let next_month_start_date = Utc
				.ymd(
					if month == 12 { year + 1 } else { year },
					if month == 12 { 1 } else { month + 1 },
					1,
				)
				.and_hms(0, 0, 0)
				.sub(Duration::nanoseconds(1));

			// Total amount due by the user for this workspace, after accounting
			// for their credits
			let amount_due = db::get_total_amount_to_pay_for_workspace(
				connection,
				&workspace.id,
			)
			.await?;

			// The total bill for the previous month, without accounting for
			// credits
			let total_bill = service::calculate_total_bill_for_workspace_till(
				connection,
				&workspace.id,
				&month_start_date,
				&next_month_start_date,
			)
			.await?;

			let card_amount_to_be_charged =
				if amount_due <= 0f64 { 0f64 } else { amount_due };

			// How much of the bill was charged from credits
			let credits_deducted =
				total_bill.total_charge.0 - card_amount_to_be_charged;

			let credits_deducted = if credits_deducted <= 0f64 {
				0f64
			} else {
				credits_deducted
			};

			// How many credits are remaining in your account
			let credits_remaining = if amount_due >= 0.00 {
				0.00
			} else {
				// If amount due is negative, the negative value implies the
				// amount of credits remaining in their account. The positive
				// value of that is the credits remaining in their account
				-amount_due
			};

			if amount_due <= 0f64 && total_bill.total_charge.0 < 0.01 {
				// No paid resources have been used. They have no bill to their
				// account. No payment needs to be made. Don't bother sending
				// them an email and annoying them.
				return Ok(());
			}

			let workspace_name = if let Some(Ok(user_id)) = workspace
				.name
				.strip_prefix("personal-workspace-")
				.map(Uuid::parse_str)
			{
				if let Some(user) =
					db::get_user_by_user_id(connection, &user_id).await?
				{
					format!(
						"{} {}'s personal workspace",
						user.first_name, user.last_name
					)
				} else {
					workspace.name.clone()
				}
			} else {
				workspace.name.clone()
			};

			if amount_due <= 0f64 {
				let billing_address = if let Some(address) =
					workspace.address_id
				{
					let address = db::get_billing_address(connection, &address)
						.await?
						.status(500)?;

					Address {
						first_name: address.first_name,
						last_name: address.last_name,
						address_line_1: address.address_line_1,
						address_line_2: address.address_line_2,
						address_line_3: address.address_line_3,
						city: address.city,
						state: address.state,
						zip: address.zip,
						country: address.country,
					}
				} else {
					Address {
						first_name: "".to_string(),
						last_name: "".to_string(),
						address_line_1: "".to_string(),
						address_line_2: None,
						address_line_3: None,
						city: "".to_string(),
						state: "".to_string(),
						zip: "".to_string(),
						country: "".to_string(),
					}
				};
				service::send_payment_success_invoice_notification(
					connection,
					&workspace.super_admin_id,
					workspace_name,
					month_string.to_string(),
					total_bill,
					billing_address,
					PriceAmount(credits_deducted),
					// If `amount_due` is 0, definitely the card amount is 0
					PriceAmount(card_amount_to_be_charged),
					PriceAmount(credits_remaining),
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
					let address =
						db::get_billing_address(connection, address_id)
							.await?
							.status(500)?;
					let billing_address = Address {
						first_name: address.first_name,
						last_name: address.last_name,
						address_line_1: address.address_line_1,
						address_line_2: address.address_line_2,
						address_line_3: address.address_line_3,
						city: address.city,
						state: address.state,
						zip: address.zip,
						country: address.country,
					};
					let (currency, stripe_amount) = if billing_address.country ==
						*"IN"
					{
						(
							Currency::INR,
							(card_amount_to_be_charged * 100f64 * 80f64) as i64,
						)
					} else {
						(
							Currency::USD,
							(card_amount_to_be_charged * 100f64) as i64,
						)
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

					// Payment will only have successful or failed state
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
							card_amount_to_be_charged,
							Some(&payment_intent.id),
							&Utc::now(),
							&TransactionType::Payment,
							&PaymentStatus::Success,
							None,
						)
						.await?;

						service::send_payment_success_invoice_notification(
							connection,
							&workspace.super_admin_id,
							workspace_name,
							month_string.to_string(),
							total_bill,
							billing_address,
							PriceAmount(credits_deducted),
							// If `amount_due` is 0, definitely the card amount
							// is 0
							PriceAmount(card_amount_to_be_charged),
							PriceAmount(credits_remaining),
						)
						.await?;

						Ok(())
					} else {
						db::create_transaction(
							connection,
							&workspace.id,
							&transaction_id,
							month as i32,
							card_amount_to_be_charged,
							Some(&payment_intent.id),
							&Utc::now(),
							&TransactionType::Payment,
							&PaymentStatus::Failed,
							None,
						)
						.await?;

						service::send_payment_failure_invoice_notification(
							connection,
							&workspace.super_admin_id,
							workspace_name,
							total_bill,
							billing_address,
							month_string.to_string(),
						)
						.await?;

						service::queue_retry_payment_for_workspace(
							&workspace.id,
							&Utc::now().add(Duration::days(1)),
							month,
							year,
							config,
						)
						.await?;

						Ok(())
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

					service::send_payment_failure_invoice_notification(
						connection,
						&workspace.super_admin_id,
						workspace_name,
						total_bill,
						Address {
							first_name: "".to_string(),
							last_name: "".to_string(),
							address_line_1: "".to_string(),
							address_line_2: None,
							address_line_3: None,
							city: "".to_string(),
							state: "".to_string(),
							zip: "".to_string(),
							country: "".to_string(),
						},
						month_string.to_string(),
					)
					.await?;

					service::queue_retry_payment_for_workspace(
						&workspace.id,
						&Utc::now().add(Duration::days(1)),
						month,
						year,
						config,
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
					total_bill.total_charge.0,
					Some("enterprise-plan-payment"),
					// 1st of next month, so that the payment will be recorded
					// on 1st of october for a bill generated for september
					// month_start_date represents the previous month bill.
					// Next month start date will signify the bill being paid
					// for on the 1st of the next month
					&next_month_start_date.add(Duration::nanoseconds(1)),
					&TransactionType::Payment,
					&PaymentStatus::Success,
					None,
				)
				.await?;

				let total_charge = total_bill.total_charge;

				let billing_address = if let Some(address) =
					workspace.address_id
				{
					let address = db::get_billing_address(connection, &address)
						.await?
						.status(500)?;

					Address {
						first_name: address.first_name,
						last_name: address.last_name,
						address_line_1: address.address_line_1,
						address_line_2: address.address_line_2,
						address_line_3: address.address_line_3,
						city: address.city,
						state: address.state,
						zip: address.zip,
						country: address.country,
					}
				} else {
					Address {
						first_name: "".to_string(),
						last_name: "".to_string(),
						address_line_1: "".to_string(),
						address_line_2: None,
						address_line_3: None,
						city: "".to_string(),
						state: "".to_string(),
						zip: "".to_string(),
						country: "".to_string(),
					}
				};

				service::send_payment_success_invoice_notification(
					connection,
					&workspace.super_admin_id,
					workspace_name,
					month_string.to_string(),
					total_bill,
					billing_address,
					total_charge,
					PriceAmount(0f64),
					PriceAmount(0f64),
				)
				.await?;

				Ok(())
			}
		}
		BillingData::RetryPaymentForWorkspace {
			workspace_id,
			process_after: DateTime(process_after),
			month,
			year,
			request_id,
		} => {
			if Utc::now() < process_after {
				// process_after is in the future. Wait for a while and requeue
				time::sleep(time::Duration::from_millis(
					if cfg!(debug_assertions) { 1000 } else { 60_000 },
				))
				.await;
				return Err(Error::empty());
			}

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

			let month_start_date =
				Utc.ymd(year as i32, month, 1).and_hms(0, 0, 0);
			let next_month_start_date = Utc
				.ymd(
					(if month == 12 { year + 1 } else { year }) as i32,
					if month == 12 { 1 } else { month + 1 },
					1,
				)
				.and_hms(0, 0, 0)
				.sub(Duration::nanoseconds(1));

			let workspace = db::get_workspace_info(connection, &workspace_id)
				.await?
				.status(500)
				.body("workspace_id should be a valid id")?;

			// Total amount due by the user for this workspace, after accounting
			// for their credits
			let amount_due = db::get_total_amount_to_pay_for_workspace(
				connection,
				&workspace.id,
			)
			.await?;

			// The total bill for the previous month, without accounting for
			// credits
			let total_bill = service::calculate_total_bill_for_workspace_till(
				connection,
				&workspace.id,
				&month_start_date,
				&next_month_start_date,
			)
			.await?;

			let card_amount_to_be_charged =
				if amount_due <= 0f64 { 0f64 } else { amount_due };

			// How much of the bill was charged from credits
			let credits_deducted =
				total_bill.total_charge.0 - card_amount_to_be_charged;

			let credits_deducted = if credits_deducted <= 0f64 {
				0f64
			} else {
				credits_deducted
			};

			// How many credits are remaining in your account
			let credits_remaining = if amount_due >= 0.00 {
				0.00
			} else {
				// If amount due is negative, the negative value implies the
				// amount of credits remaining in their account. The positive
				// value of that is the credits remaining in their account
				-amount_due
			};

			let workspace_name = if let Some(Ok(user_id)) = workspace
				.name
				.strip_prefix("personal-workspace-")
				.map(Uuid::parse_str)
			{
				if let Some(user) =
					db::get_user_by_user_id(connection, &user_id).await?
				{
					format!(
						"{} {}'s personal workspace",
						user.first_name, user.last_name
					)
				} else {
					workspace.name.clone()
				}
			} else {
				workspace.name.clone()
			};

			if amount_due <= 0f64 {
				let billing_address = if let Some(address) =
					workspace.address_id
				{
					let address = db::get_billing_address(connection, &address)
						.await?
						.status(500)?;

					Address {
						first_name: address.first_name,
						last_name: address.last_name,
						address_line_1: address.address_line_1,
						address_line_2: address.address_line_2,
						address_line_3: address.address_line_3,
						city: address.city,
						state: address.state,
						zip: address.zip,
						country: address.country,
					}
				} else {
					Address {
						first_name: "".to_string(),
						last_name: "".to_string(),
						address_line_1: "".to_string(),
						address_line_2: None,
						address_line_3: None,
						city: "".to_string(),
						state: "".to_string(),
						zip: "".to_string(),
						country: "".to_string(),
					}
				};
				service::send_payment_success_invoice_notification(
					connection,
					&workspace.super_admin_id,
					workspace_name,
					month_string.to_string(),
					total_bill,
					billing_address,
					PriceAmount(credits_deducted),
					// If `amount_due` is 0, definitely the card amount is 0
					PriceAmount(card_amount_to_be_charged),
					PriceAmount(credits_remaining),
				)
				.await?;
				return Ok(());
			}

			let card_added;

			if let (Some(address_id), Some(payment_method_id)) =
				(&workspace.address_id, &workspace.default_payment_method_id)
			{
				let (currency, stripe_amount) =
					if db::get_billing_address(connection, address_id)
						.await?
						.status(500)?
						.country == *"IN"
					{
						(
							Currency::INR,
							(card_amount_to_be_charged * 100f64 * 80f64) as i64,
						)
					} else {
						(
							Currency::USD,
							(card_amount_to_be_charged * 100f64) as i64,
						)
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
					intent.off_session = Some(PaymentIntentOffSession::Other(
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

				// Payment will only have successful or failed state
				// according to stripe docs. For more information.
				// check - https://stripe.com/docs/payments/payment-intents/verifying-status
				if let PaymentIntentStatus::Succeeded = payment_intent.status {
					db::create_transaction(
						connection,
						&workspace.id,
						&transaction_id,
						month as i32,
						card_amount_to_be_charged,
						Some(&payment_intent.id),
						&Utc::now(),
						&TransactionType::Payment,
						&PaymentStatus::Success,
						None,
					)
					.await?;

					service::send_bill_paid_successfully_email(
						connection,
						workspace.super_admin_id,
						workspace_name,
						month_string.to_string(),
						year,
						PriceAmount(card_amount_to_be_charged),
					)
					.await?;

					return Ok(());
				}

				// If not successfully charged, then this
				db::create_transaction(
					connection,
					&workspace.id,
					&transaction_id,
					month as i32,
					card_amount_to_be_charged,
					Some(&payment_intent.id),
					&Utc::now(),
					&TransactionType::Payment,
					&PaymentStatus::Failed,
					None,
				)
				.await?;

				card_added = true;
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

				card_added = false;
			}

			// notify customer that the payment has failed
			if (Utc::now() - next_month_start_date) > Duration::days(15) {
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
				service::send_bill_not_paid_delete_resources_email(
					connection,
					workspace.super_admin_id.clone(),
					workspace_name,
					month_string.to_string(),
					month,
					year,
					card_amount_to_be_charged,
				)
				.await?;

				Ok(())
			} else {
				let deadline_day = 15;
				let next_month = if month == 12 { 1 } else { month + 1 };

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

				if card_added {
					service::send_bill_payment_failed_reminder_email(
						connection,
						workspace.super_admin_id.clone(),
						workspace_name,
						month_string.to_string(),
						month,
						year,
						// If `amount_due` is 0, definitely the card amount
						// is 0
						total_bill.total_charge,
						deadline,
					)
					.await?;
				} else {
					service::send_card_not_added_reminder_email(
						connection,
						workspace.super_admin_id.clone(),
						workspace_name,
						month_string.to_string(),
						month,
						year,
						// If `amount_due` is 0, definitely the card amount
						// is 0
						total_bill.total_charge,
						deadline,
					)
					.await?;
				}

				service::queue_retry_payment_for_workspace(
					&workspace.id,
					&Utc::now().add(Duration::days(1)),
					month,
					year,
					config,
				)
				.await?;

				Ok(())
			}
		}
	}
}
