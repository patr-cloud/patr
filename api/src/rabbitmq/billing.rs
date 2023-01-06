use std::{
	cmp::max,
	ops::{Add, Sub},
	str::FromStr,
};

use api_models::{
	models::workspace::billing::{
		Address,
		PaymentStatus,
		TotalAmount,
		TransactionType,
	},
	utils::{DateTime, Uuid},
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
	PaymentIntentStatus,
	PaymentMethodId,
	RequestStrategy,
};
use tokio::time;

use crate::{
	db::{self, PaymentType},
	models::rabbitmq::BillingData,
	service,
	utils::{billing::stringify_month, settings::Settings, Error},
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
			if Utc::now() <
				Utc.with_ymd_and_hms(year, month, 1, 0, 0, 0).unwrap()
			{
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
			let month_start_date =
				Utc.with_ymd_and_hms(year, month, 1, 0, 0, 0).unwrap();
			let next_month_start_date = Utc
				.with_ymd_and_hms(
					if month == 12 { year + 1 } else { year },
					if month == 12 { 1 } else { month + 1 },
					1,
					0,
					0,
					0,
				)
				.unwrap()
				.sub(Duration::nanoseconds(1));

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
				total_bill_breakdown.total_charge,
				None,
				// 1st of next month,
				&next_month_start_date.add(Duration::nanoseconds(1)),
				&TransactionType::Bill,
				&PaymentStatus::Success,
				Some(&format!(
					"Bill for {} {}",
					stringify_month(month as u8),
					year
				)),
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

			let month_start_date =
				Utc.with_ymd_and_hms(year, month, 1, 0, 0, 0).unwrap();
			let next_month_start_date = Utc
				.with_ymd_and_hms(
					if month == 12 { year + 1 } else { year },
					if month == 12 { 1 } else { month + 1 },
					1,
					0,
					0,
					0,
				)
				.unwrap()
				.sub(Duration::nanoseconds(1));

			// Total amount due by the user for this workspace, after accounting
			// for their credits
			let amount_due_in_cents =
				db::get_total_amount_in_cents_to_pay_for_workspace(
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

			let (
				card_amount_to_be_charged_in_cents,
				credits_deducted_in_cents,
				credits_remaining_in_cents,
			) = match amount_due_in_cents {
				TotalAmount::CreditsLeft(credits_left) => {
					(0, total_bill.total_charge, credits_left)
				}
				TotalAmount::NeedToPay(need_to_pay) => (
					need_to_pay,
					total_bill.total_charge.saturating_sub(need_to_pay), /* if total charge is 0 we cannot
					                                                      * subtract u64 from 0 */
					0,
				),
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

			if card_amount_to_be_charged_in_cents == 0 &&
				total_bill.total_charge == 0
			{
				// do nothing, no email has to be sent for any payment action
				log::trace!(
					"request_id: {} Nothing charged for workspace: {}",
					request_id,
					workspace.id
				);
				return Ok(());
			} else if card_amount_to_be_charged_in_cents == 0 &&
				total_bill.total_charge > 0
			{
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

				// Only if amount due is zero or total resource usage is zero
				service::send_payment_success_invoice_notification(
					connection,
					&workspace.super_admin_id,
					workspace_name,
					total_bill,
					billing_address,
					credits_deducted_in_cents,
					// If `amount_due` is 0, definitely the card amount is 0
					card_amount_to_be_charged_in_cents,
					credits_remaining_in_cents,
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
					log::trace!("request_id: {} found address and card for workspace_name: {}", request_id, workspace.id);
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
						(Currency::INR, card_amount_to_be_charged_in_cents * 80)
					} else {
						(Currency::USD, card_amount_to_be_charged_in_cents)
					};

					let payment_description = Some(format!(
						"Patr charge: Bill for {} {}",
						stringify_month(month as u8),
						year
					));

					log::trace!(
						"request_id: {} attempting stripe call",
						request_id
					);

					let payment_intent = PaymentIntent::create(
						&Client::new(&config.stripe.secret_key).with_strategy(
							RequestStrategy::Idempotent(format!(
								"{}-{}-{}",
								workspace.id, month, year
							)),
						),
						{
							let mut intent = CreatePaymentIntent::new(
								stripe_amount as i64,
								currency,
							);

							intent.confirm = Some(true);
							intent.confirmation_method = Some(
								PaymentIntentConfirmationMethod::Automatic,
							);
							intent.off_session =
								Some(PaymentIntentOffSession::Other(
									OffSessionOther::OneOff,
								));
							intent.description = payment_description.as_deref();
							intent.customer = Some(CustomerId::from_str(
								&workspace.stripe_customer_id,
							)?);
							intent.payment_method = Some(
								PaymentMethodId::from_str(payment_method_id)?,
							);
							intent.payment_method_types =
								Some(vec!["card".to_string()]);

							intent
						},
					)
					.await
					.map_err(|err| {
						log::warn!(
							"Error while creating payment indent: {err:#?}"
						);
						err
					})?;

					let transaction_id =
						db::generate_new_transaction_id(connection).await?;

					// Payment will only have successful or failed state
					// according to stripe docs. For more information.
					// check - https://stripe.com/docs/payments/payment-intents/verifying-status
					if let PaymentIntentStatus::Succeeded =
						payment_intent.status
					{
						log::trace!(
							"request_id: {} made success payment with
					payment intent id {}",
							request_id,
							payment_intent.id
						);
						db::create_transaction(
							connection,
							&workspace.id,
							&transaction_id,
							month as i32,
							card_amount_to_be_charged_in_cents,
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
							total_bill,
							billing_address,
							credits_deducted_in_cents,
							// If `amount_due` is 0, definitely the card amount
							// is 0
							card_amount_to_be_charged_in_cents,
							credits_remaining_in_cents,
						)
						.await?;

						Ok(())
					} else {
						log::trace!(
							"request_id: {} payment failed",
							request_id
						);
						db::create_transaction(
							connection,
							&workspace.id,
							&transaction_id,
							month as i32,
							card_amount_to_be_charged_in_cents,
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
					)
					.await?;

					log::trace!(
						"request_id: {} retrying payment for workspace {}",
						request_id,
						workspace.id
					);

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
				log::trace!("Enterprise plan, assume the payment is made");
				let transaction_id =
					db::generate_new_transaction_id(connection).await?;
				db::create_transaction(
					connection,
					&workspace.id,
					&transaction_id,
					month as i32,
					total_bill.total_charge,
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
					total_bill,
					billing_address,
					total_charge,
					0,
					0,
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

			let month_start_date = Utc
				.with_ymd_and_hms(year as i32, month, 1, 0, 0, 0)
				.unwrap();

			let next_month_start_date = Utc
				.with_ymd_and_hms(
					(if month == 12 { year + 1 } else { year }) as i32,
					if month == 12 { 1 } else { month + 1 },
					1,
					0,
					0,
					0,
				)
				.unwrap()
				.sub(Duration::nanoseconds(1));

			let workspace = db::get_workspace_info(connection, &workspace_id)
				.await?
				.status(500)
				.body("workspace_id should be a valid id")?;

			// Total amount due by the user for this workspace, after accounting
			// for their credits
			let amount_due_in_cents =
				db::get_total_amount_in_cents_to_pay_for_workspace(
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

			let (
				card_amount_to_be_charged_in_cents,
				_credits_deducted_in_cents,
				_credits_remaining_in_cents,
			) = match amount_due_in_cents {
				TotalAmount::CreditsLeft(credits_left) => {
					(0, total_bill.total_charge, credits_left)
				}
				TotalAmount::NeedToPay(need_to_pay) => (
					need_to_pay,
					max(0, total_bill.total_charge - need_to_pay),
					0,
				),
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

			if card_amount_to_be_charged_in_cents == 0 {
				// the notification for payment success will be sent whenever
				// the user has made a payment, so no need to send it again when
				// confirming it in rabbitmq

				return Ok(());
			}

			let card_added;

			if let (Some(address_id), Some(payment_method_id)) =
				(&workspace.address_id, &workspace.default_payment_method_id)
			{
				log::trace!("request_id: {} found card, retrying payment for workspace: {}", request_id, workspace_id);
				let (currency, stripe_amount) =
					if db::get_billing_address(connection, address_id)
						.await?
						.status(500)?
						.country == *"IN"
					{
						(Currency::INR, card_amount_to_be_charged_in_cents * 80)
					} else {
						(Currency::USD, card_amount_to_be_charged_in_cents)
					};

				let payment_description = Some(format!(
					"Patr charge: Bill for {} {}",
					stringify_month(month as u8),
					year
				));
				let payment_intent = PaymentIntent::create(
					&Client::new(&config.stripe.secret_key).with_strategy(
						RequestStrategy::Idempotent(format!(
							"{}-{}-{}",
							workspace.id, month, year
						)),
					),
					{
						let mut intent = CreatePaymentIntent::new(
							stripe_amount as i64,
							currency,
						);

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

						intent
					},
				)
				.await
				.map_err(|err| {
					log::warn!("Error while creating payment indent: {err:#?}");
					err
				})?;

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
						card_amount_to_be_charged_in_cents,
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
						month,
						year,
						card_amount_to_be_charged_in_cents,
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
					card_amount_to_be_charged_in_cents,
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
					month,
					year,
					card_amount_to_be_charged_in_cents,
				)
				.await?;

				Ok(())
			} else {
				let deadline_day = 15;
				let next_month = if month == 12 { 1 } else { month + 1 };

				let next_month_name_str = stringify_month(next_month as u8);

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
