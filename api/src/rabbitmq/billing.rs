use std::collections::HashMap;

use chrono::{Datelike, Duration, TimeZone, Utc};
use eve_rs::AsError;

use crate::{
	db::{self, get_all_workspaces},
	error,
	models::rabbitmq::WorkspaceRequestData,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn process_request(
	connection: &mut <Database as sqlx::Database>::Connection,
	request_data: WorkspaceRequestData,
	config: &Settings,
) -> Result<(), Error> {
	match request_data {
		WorkspaceRequestData::GenerateInvoice { month, year } => {
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

			if current_month < month.into() {
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
			let workspaces = get_all_workspaces(connection).await?;

			for workspace in workspaces {
				// get the resources
				// calculate the cost
				// store the transaction
				let start_date =
					Utc.ymd(year as i32, month, 1).and_hms(0, 0, 0);
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
						));
				});

				let deployments = db::get_deployments_for_workspace(
					connection,
					&workspace.id,
				)
				.await?;

				let mut total_cost = 0f64;

				let mut price_distribution = HashMap::new();

				// only deployments at this point of time
				for d in deployments {
					if resource_map.contains_key(&d.id) {
						let resource_vec =
							resource_map.get(&d.id).status(500)?;

						for (pos, (date, price, quantity, active)) in
							resource_vec.iter().enumerate()
						{
							if *active {
								if pos + 1 == resource_vec.len() {
									let hours =
										(end_date.timestamp() - date) / 3600;

									price_distribution.insert(
										d.id.clone(),
										(hours, price, quantity),
									);

									total_cost = total_cost +
										(hours as f64 *
											price * *quantity as f64);
								} else {
									let (date2, ..) = resource_vec[pos + 1];

									let hours = (date2 - date) / 3600;

									price_distribution.insert(
										d.id.clone(),
										(hours, price, quantity),
									);

									total_cost = total_cost +
										(hours as f64 *
											price * *quantity as f64);
								}
							}
						}
					}
				}

				// send the total_cost to stripe

				// send the invoice to the user

				// create a transaction for the invoice with payment id
			}
		}
		WorkspaceRequestData::ChargeUser {} => {
			// charge the user using the stripe api

			// update transaction with payment id and status of transaction
			// maybe?
		}
	}
	Ok(())
}
