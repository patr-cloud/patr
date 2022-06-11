use std::collections::HashMap;

use chrono::{Datelike, Duration, TimeZone, Utc};
use eve_rs::AsError;
use reqwest::Client;

use crate::{
	db::{self, get_all_workspaces},
	error,
	models::{
		deployment,
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
		WorkspaceRequestData::ProcessWorkspace { month, year } => {
			let workspaces = get_all_workspaces(connection).await?;

			for workspace in workspaces {
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
		} => {
			// generate and invoice for the month
			// send the invoice to the user

			// maybe do this in a loop?
			// enqueue another task of stripe
			// to charge the user for the previous month

			// send ack

			// check time
			// if it is not the end of the month
			// then send nack
			let current_month = Utc::now().month();
			let current_year = Utc::now().year();

			if (current_month < month.into() && current_year < year) ||
				(current_month > month.into() && current_year < year)
			{
				// send nack
				// change the error
				return Error::as_result()
					.status(500)
					.body(error!(SERVER_ERROR).to_string())?;
			}

			// go through all the workspaces
			// and check the resources that they are using
			// calculate the cost of each resource
			// store it as a transaction

			// get the resources
			// calculate the cost
			// store the transaction
			let start_date = Utc.ymd(year as i32, month, 1).and_hms(0, 0, 0);
			let end_date = if month == 12 {
				Utc.ymd((year + 1) as i32, 1, 1).and_hms(0, 0, 0) -
					Duration::seconds(1)
			} else {
				Utc.ymd(year as i32, month + 1, 1).and_hms(0, 0, 0) -
					Duration::seconds(1)
			};

			let mut resource_map = HashMap::new();

			db::get_billable_services(
				connection,
				&workspace.id,
				start_date.into(),
				end_date.into(),
			)
			.await?
			.into_iter()
			.for_each(|resource| {
				resource_map
					.entry(resource.resource_id)
					.or_insert(Vec::new())
					.push((
						resource.date.timestamp(),
						resource.price,
						resource.quantity,
						resource.active,
						resource.deployment_machine_type_id,
						resource.resource_type,
					));
			});

			let deployments =
				db::get_deployments_for_workspace(connection, &workspace.id)
					.await?;

			let mut total_cost = 0f64;

			let mut price_distribution = HashMap::new();

			// only deployments at this point of time
			for d in deployments {
				if resource_map.contains_key(&d.id) {
					let resource_vec = resource_map.get(&d.id).status(500)?;

					for (
						pos,
						(
							date,
							price,
							quantity,
							active,
							machine_type_id,
							resource_type,
						),
					) in resource_vec.iter().enumerate()
					{
						if *active {
							let machine_type_id = if let Some(machine_type_id) =
								machine_type_id
							{
								let (cpu_count, memory_count) =
									deployment::MACHINE_TYPES
										.get()
										.unwrap()
										.get(&machine_type_id)
										.unwrap_or(&(1, 2));
								format!(
									"{} CPU, {} GB RAM",
									cpu_count, memory_count
								)
							} else {
								"".to_string()
							};

							if pos + 1 == resource_vec.len() {
								let hours =
									(end_date.timestamp() - date) / 3600;

								price_distribution.insert(
									d.id.clone(),
									(
										hours,
										*price,
										*quantity,
										machine_type_id,
										resource_type,
									),
								);

								total_cost = total_cost +
									(hours as f64 * price * *quantity as f64);
							} else {
								let (date2, ..) = resource_vec[pos + 1];

								let hours = (date2 - date) / 3600;

								price_distribution.insert(
									d.id.clone(),
									(
										hours,
										*price,
										*quantity,
										machine_type_id,
										resource_type,
									),
								);

								total_cost = total_cost +
									(hours as f64 * price * *quantity as f64);
							}
						}
					}
				}
			}

			let client = Client::new();

			let password: Option<String> = None;

			let payment_method =
				workspace.primary_payment_method.status(500)?;

			let payment_intent = client
				.post("https://api.stripe.com/v1/payment_intents")
				.basic_auth(&config.stripe.secret_key, password)
				.form(&PaymentIntent {
					amount: total_cost,
					currency: "usd".to_string(),
					payment_method,
					payment_method_types: "card".to_string(),
					customer: workspace.id.to_string(),
				})
				.send()
				.await?
				.json::<PaymentIntentObject>()
				.await?;

			// TODO: find a better way to do this
			// TODO: create invoice template
			// send the invoice to the user
			service::send_invoice_email(
				connection,
				month,
				year,
				&workspace.super_admin_id,
				price_distribution,
				total_cost,
			)
			.await?;
			// charge the user using the stripe api
			super::queue_confirm_payment_intent(config, payment_intent.id).await

			// update transaction with payment id and status of transaction
			// maybe?
			// let transaction_id =
			// db::generate_new_transaction_id(connection).await?;
			// db::create_transaction(connection, &transaction_id, ).await?;
			// create a transaction for the invoice with payment id
		}
		WorkspaceRequestData::ConfirmPaymentIntent { payment_intent_id } => {
			// confirming payment intent and charging the user

			let client = Client::new();

			let password: Option<String> = None;

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
				return Error::as_result()
					.status(500)
					.body(error!(SERVER_ERROR).to_string())?;
			}

			Ok(())
		}
	}
}
