use std::collections::HashMap;

use api_models::utils::DateTime;
use chrono::{Datelike, Duration, TimeZone, Utc};
use eve_rs::AsError;
use reqwest::Client;

use crate::{
	db::{self, get_all_workspaces, ManagedDatabasePlan, TransactionType},
	error,
	models::{
		deployment,
		rabbitmq::WorkspaceRequestData,
		PaymentIntent,
		PaymentIntentObject,
	},
	service,
	utils::{limits::free_limits, settings::Settings, Error},
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
				println!("Processing workspace {}", workspace.id);
				println!("month: {}, year: {}", month, year);
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
			log::trace!("getting current date");
			let current_month = Utc::now().month();
			let current_year = Utc::now().year();

			// if (current_month < month && current_year < year) ||
			// 	(current_month > month && current_year < year) ||
			// 	(current_month == month && current_year == year) |
			// 		(current_month < month && current_year == year)
			// {
			// 	// send nack
			// 	// change the error
			// 	return Error::as_result()
			// 		.status(500)
			// 		.body(error!(SERVER_ERROR).to_string())?;
			// }

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

			let end_date = Utc::now();

			let mut resource_map = HashMap::new();

			log::trace!("fetching billable services for all resources");
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
						resource.deployment_machine_type,
						resource.resource_type,
					));
			});

			let deployments = db::get_all_deployments_for_workspace(
				connection,
				&workspace.id,
			)
			.await?;

			let mut deployment_cost = 0f64;

			let mut price_distribution = HashMap::new();

			log::trace!("calculating the bill for all the deployments");
			for d in deployments.clone() {
				if resource_map.contains_key(&d.id) {
					let resource_vec = resource_map.get(&d.id).status(500)?;
					println!("TEST1");
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
						println!("TEST2");
						let quantity = quantity.unwrap_or(0);

						println!("TEST3");

						if *active {
							let machine_type_id = if let Some(machine_type_id) =
								machine_type_id
							{
								println!("TEST4");
								let (cpu_count, memory_count) =
									deployment::MACHINE_TYPES
										.get()
										.unwrap()
										.get(machine_type_id)
										.unwrap_or(&(1, 2));
								format!(
									"{} CPU, {:.1} GB RAM",
									cpu_count,
									(*memory_count as f64) / 4f64
								)
							} else {
								"".to_string()
							};

							println!("TEST5");
							if pos + 1 == resource_vec.len() {
								println!("TEST6");
								let hours =
									(end_date.timestamp() - date) / 3600;

								price_distribution.insert(
									d.id.clone(),
									(
										hours,
										*price,
										quantity,
										machine_type_id,
										resource_type,
									),
								);

								deployment_cost +=
									hours as f64 * price * quantity as f64;
							} else {
								println!("TEST7");
								let (date2, ..) = resource_vec[pos + 1];

								let hours = (date2 - date) / 3600;

								price_distribution.insert(
									d.id.clone(),
									(
										hours,
										*price,
										quantity,
										machine_type_id,
										resource_type,
									),
								);

								deployment_cost +=
									hours as f64 * price * quantity as f64;
							}
						}

						println!("TEST8");
						let machine_type_id =
							if let Some(machine_type_id) = machine_type_id {
								let (cpu_count, memory_count) =
									deployment::MACHINE_TYPES
										.get()
										.unwrap()
										.get(machine_type_id)
										.unwrap_or(&(1, 2));
								format!(
									"{} CPU, {:.1} GB RAM",
									cpu_count,
									(*memory_count as f64) / 4f64
								)
							} else {
								"".to_string()
							};

						println!("TEST9");
						if deployments.len() == 1 &&
							machine_type_id ==
								"1 CPU, 0.5 GB RAM".to_string()
						{
							deployment_cost = 0f64;
						}
					}
				}
				println!("TEST10");
			}

			println!("TEST11");
			let static_sites =
				db::get_static_sites_for_workspace(connection, &workspace.id)
					.await?;

			println!("TEST12");
			let static_site_product_id =
				db::get_product_info_by_name(connection, "static-site")
					.await?
					.status(500)?
					.id;

			println!("TEST13");
			let active_plan = db::get_active_plan_for_product_in_workspace(
				connection,
				&workspace.id,
				&static_site_product_id,
			)
			.await?
			.status(500)?;

			println!("TEST14");
			let plan_info =
				db::get_plan_by_id(connection, &active_plan.plan_id)
					.await?
					.status(500)?;

			println!("TEST15");
			let static_site_cost = if let Some(quantity) = plan_info.quantity {
				if static_sites.len() as u32 > *free_limits::FREE_STATIC_SITES &&
					static_sites.len() <= quantity as usize
				{
					5f64
				} else if static_sites.len() > quantity as usize {
					10f64
				} else {
					0f64
				}
			} else {
				10f64
			};

			println!("TEST16");
			let managed_databases =
				db::get_all_database_clusters_for_workspace(
					connection,
					&workspace.id,
				)
				.await?;

			println!("TEST17");
			let managed_database_product_id =
				db::get_product_info_by_name(connection, "managed-database")
					.await?
					.status(500)?
					.id;

			println!("TEST18");
			let mut managed_database_cost = 0f64;

			println!("TEST19");
			for managed_database in managed_databases {
				println!("TEST20");
				match managed_database.database_plan {
					ManagedDatabasePlan::Nano => {
						println!("TEST22");
						let plan_info = db::get_plan_by_name(
							connection,
							"managed-database-nano",
							&workspace.id,
						)
						.await?
						.status(500)?;
						managed_database_cost += plan_info.price;
					}
					ManagedDatabasePlan::Micro => {
						println!("TEST23");
						let plan_info = db::get_plan_by_name(
							connection,
							"managed-database-micro",
							&workspace.id,
						)
						.await?
						.status(500)?;
						managed_database_cost += plan_info.price;
					}
					ManagedDatabasePlan::Small => {
						println!("TEST24");
						let plan_info = db::get_plan_by_name(
							connection,
							"managed-database-small",
							&workspace.id,
						)
						.await?
						.status(500)?;
						managed_database_cost += plan_info.price;
					}
					ManagedDatabasePlan::Medium => {
						println!("TEST25");
						let plan_info = db::get_plan_by_name(
							connection,
							"managed-database-medium",
							&workspace.id,
						)
						.await?
						.status(500)?;
						managed_database_cost += plan_info.price;
					}
					ManagedDatabasePlan::Large => {
						println!("TEST26");
						let plan_info = db::get_plan_by_name(
							connection,
							"managed-database-large",
							&workspace.id,
						)
						.await?
						.status(500)?;
						managed_database_cost += plan_info.price;
					}
					ManagedDatabasePlan::Xlarge => {
						println!("TEST27");
						let plan_info = db::get_plan_by_name(
							connection,
							"managed-database-xlarge",
							&workspace.id,
						)
						.await?
						.status(500)?;
						managed_database_cost += plan_info.price;
					}
					ManagedDatabasePlan::Xxlarge => {
						println!("TEST28");
						let plan_info = db::get_plan_by_name(
							connection,
							"managed-database-xxlarge",
							&workspace.id,
						)
						.await?
						.status(500)?;
						managed_database_cost += plan_info.price;
					}
					ManagedDatabasePlan::Mammoth => {
						println!("TEST29");
						let plan_info = db::get_plan_by_name(
							connection,
							"managed-database-mammoth",
							&workspace.id,
						)
						.await?
						.status(500)?;
						managed_database_cost += plan_info.price;
					}
				}
			}

			println!("TEST30");
			let managed_urls = db::get_all_managed_urls_in_workspace(
				connection,
				&workspace.id,
			)
			.await?;

			println!("TEST31");
			let managed_url_product_id =
				db::get_product_info_by_name(connection, "managed-url")
					.await?
					.status(500)?
					.id;

			println!("TEST32");
			let active_plan = db::get_active_plan_for_product_in_workspace(
				connection,
				&workspace.id,
				&managed_url_product_id,
			)
			.await?
			.status(500)?;

			println!("TEST33");
			let plan_info =
				db::get_plan_by_id(connection, &active_plan.plan_id)
					.await?
					.status(500)?;

			println!("TEST34");
			let managed_url_cost = if let Some(quantity) = plan_info.quantity {
				if managed_urls.len() as u32 > *free_limits::FREE_MANAGED_URLS &&
					managed_urls.len() <= quantity as usize
				{
					10f64
				} else if managed_urls.len() > quantity as usize {
					(((managed_urls.len() as f64) / 100f64) * 10f64).ceil()
				} else {
					0f64
				}
			} else {
				// TODO: modify this error to prevent it from requeuing
				return Error::as_result()
					.status(500)
					.body(error!(SERVER_ERROR).to_string())?;
			};

			println!("TEST35");
			let secrets =
				db::get_all_secrets_in_workspace(connection, &workspace.id)
					.await?;

			println!("TEST36");
			let secret_product_id =
				db::get_product_info_by_name(connection, "secret")
					.await?
					.status(500)?
					.id;

			println!("TEST37");
			let active_plan = db::get_active_plan_for_product_in_workspace(
				connection,
				&workspace.id,
				&secret_product_id,
			)
			.await?
			.status(500)?;

			println!("TEST38");
			let plan_info =
				db::get_plan_by_id(connection, &active_plan.plan_id)
					.await?
					.status(500)?;

			println!("TEST39");
			let secret_cost = if let Some(quantity) = plan_info.quantity {
				if secrets.len() as u32 > *free_limits::FREE_SECRETS &&
					secrets.len() <= quantity as usize
				{
					10f64
				} else if secrets.len() > quantity as usize {
					(((secrets.len() as f64) / 100f64) * 10f64).ceil()
				} else {
					0f64
				}
			} else {
				// TODO: modify this error to prevent it from requeuing
				return Error::as_result()
					.status(500)
					.body(error!(SERVER_ERROR).to_string())?;
			};

			println!("TEST40");
			let domains =
				db::get_domains_for_workspace(connection, &workspace.id)
					.await?;

			println!("TEST41");
			let domain_product_id =
				db::get_product_info_by_name(connection, "domain")
					.await?
					.status(500)?
					.id;

			println!("TEST42");

			let active_plan = db::get_active_plan_for_product_in_workspace(
				connection,
				&workspace.id,
				&domain_product_id,
			)
			.await?
			.status(500)?;

			println!("TEST42");
			let plan_info =
				db::get_plan_by_id(connection, &active_plan.plan_id)
					.await?
					.status(500)?;

			println!("TEST43");
			let domain_cost = if plan_info.quantity.is_some() {
				if domains.len() as u32 > *free_limits::FREE_DOMAINS {
					10f64
				} else {
					0f64
				}
			} else {
				// TODO: modify this error to prevent it from requeuing
				return Error::as_result()
					.status(500)
					.body(error!(SERVER_ERROR).to_string())?;
			};

			println!("TEST44");
			let client = Client::new();

			let password: Option<String> = None;

			let mut total_cost = deployment_cost +
				static_site_cost + managed_database_cost +
				managed_url_cost + secret_cost +
				domain_cost;

			let credits =
				db::get_credit_balance(connection, &workspace.id).await?;

			if credits.credits > total_cost {
				total_cost = credits.credits - total_cost;
			} else {
				total_cost = total_cost - credits.credits;
			}

			println!("TEST45");
			if let Some(payment_method) = workspace.primary_payment_method.clone() {
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
				println!("TEST46");

				// charge the user using the stripe api
				super::queue_confirm_payment_intent(config, payment_intent.id)
					.await?;
				println!("TEST88");
			}

			if total_cost > 0f64 && workspace.primary_payment_method.is_none() {
				log::error!("No payment method set for workspace {} and the bill is greater than 0 : {}", workspace.id, total_cost);
			}

			println!("TEST46");
			let current_time = &DateTime::from(Utc::now());

			let transaction_id =
				db::generate_new_transaction_id(connection).await?;

			db::create_transaction(
				connection,
				&transaction_id,
				None,
				&TransactionType::Onetime,
				None,
				credits.credits,
				None,
				&workspace.id,
				&DateTime::from(Utc::now()),
				None,
				true,
			)
			.await?;

			// TODO: find a better way to do this
			// TODO: create invoice template
			// send the invoice to the user
			println!("TEST47");
			service::send_invoice_email(
				connection,
				month,
				year,
				&workspace.super_admin_id,
				price_distribution,
				Some(credits.credits),
				deployment_cost,
			)
			.await?;

			println!("TEST49");
			// update transaction with payment id and status of transaction
			// maybe?
			let transaction_id =
				db::generate_new_transaction_id(connection).await?;

			let deployment_product_id =
				db::get_product_info_by_name(connection, "deployment")
					.await?
					.status(500)?
					.id;

			println!("TEST50");
			db::create_transaction(
				connection,
				&transaction_id,
				Some(&deployment_product_id),
				&TransactionType::DynamicMonthly,
				None,
				deployment_cost,
				None,
				&workspace.id,
				current_time,
				None,
				true,
			)
			.await?;

			let transaction_id =
				db::generate_new_transaction_id(connection).await?;

			db::create_transaction(
				connection,
				&transaction_id,
				Some(&static_site_product_id),
				&TransactionType::FixedMonthly,
				None,
				static_site_cost,
				None,
				&workspace.id,
				current_time,
				None,
				true,
			)
			.await?;

			let transaction_id =
				db::generate_new_transaction_id(connection).await?;

			db::create_transaction(
				connection,
				&transaction_id,
				Some(&managed_database_product_id),
				&TransactionType::DynamicMonthly,
				None,
				managed_database_cost,
				None,
				&workspace.id,
				current_time,
				None,
				true,
			)
			.await?;

			let transaction_id =
				db::generate_new_transaction_id(connection).await?;

			db::create_transaction(
				connection,
				&transaction_id,
				Some(&managed_url_product_id),
				&TransactionType::FixedMonthly,
				None,
				managed_url_cost,
				None,
				&workspace.id,
				current_time,
				None,
				true,
			)
			.await?;

			let transaction_id =
				db::generate_new_transaction_id(connection).await?;

			db::create_transaction(
				connection,
				&transaction_id,
				Some(&secret_product_id),
				&TransactionType::FixedMonthly,
				None,
				secret_cost,
				None,
				&workspace.id,
				current_time,
				None,
				true,
			)
			.await?;

			let transaction_id =
				db::generate_new_transaction_id(connection).await?;

			db::create_transaction(
				connection,
				&transaction_id,
				Some(&domain_product_id),
				&TransactionType::FixedMonthly,
				None,
				domain_cost,
				None,
				&workspace.id,
				current_time,
				None,
				true,
			)
			.await?;
			// create a transaction for the invoice with payment id

			let deployments = db::get_running_deployments_for_workspace(
				connection,
				&workspace.id,
			)
			.await?;

			for deployment in deployments {
				service::create_billable_service_for_deployment(
					connection,
					&workspace.id,
					&deployment.id,
					true,
				)
				.await?;
			}
			Ok(())
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
