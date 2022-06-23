use chrono::{Datelike, Utc};

use super::Job;
use crate::{db, service, utils::Error};

// Every two hours
pub(super) fn update_bill_job() -> Job {
	Job::new(
		String::from("Update bills of workspaces"),
		"0 0 1/2 * * *".parse().unwrap(),
		|| Box::pin(update_bill()),
	)
}

async fn update_bill() -> Result<(), Error> {
	let mut connection = super::CONFIG.get().unwrap().database.begin().await?;
	let workspaces = db::get_all_workspaces(&mut connection).await?;
	let now = Utc::now();
	let month_start_date = now.date().with_day(1).unwrap().and_hms(0, 0, 0);

	for workspace in workspaces {
		let existing_bill = db::get_total_amount_due_for_workspace(
			&mut connection,
			&workspace.id,
		)
		.await? as f64;
		let deployment_usages =
			service::calculate_deployment_bill_for_workspace_till(
				&mut connection,
				&workspace.id,
				&month_start_date,
				&now,
			)
			.await?;

		let database_usages =
			service::calculate_database_bill_for_workspace_till(
				&mut connection,
				&workspace.id,
				&month_start_date,
				&now,
			)
			.await?;

		let static_sites_usages =
			service::calculate_static_sites_bill_for_workspace_till(
				&mut connection,
				&workspace.id,
				&month_start_date,
				&now,
			)
			.await?;

		let managed_url_usages =
			service::calculate_managed_urls_bill_for_workspace_till(
				&mut connection,
				&workspace.id,
				&month_start_date,
				&now,
			)
			.await?;

		let docker_repository_usages =
			service::calculate_docker_repository_bill_for_workspace_till(
				&mut connection,
				&workspace.id,
				&month_start_date,
				&now,
			)
			.await?;

		let domains_usages =
			service::calculate_domains_bill_for_workspace_till(
				&mut connection,
				&workspace.id,
				&month_start_date,
				&now,
			)
			.await?;

		let secrets_usages =
			service::calculate_secrets_bill_for_workspace_till(
				&mut connection,
				&workspace.id,
				&month_start_date,
				&now,
			)
			.await?;

		let new_bill = {
			deployment_usages
				.iter()
				.map(|(_, bill)| {
					bill.bill_items.iter().map(|item| item.amount).sum::<f64>()
				})
				.sum::<f64>()
		} + {
			database_usages
				.iter()
				.map(|(_, bill)| bill.amount)
				.sum::<f64>()
		} + {
			static_sites_usages
				.iter()
				.map(|(_, bill)| bill.amount)
				.sum::<f64>()
		} + {
			managed_url_usages
				.iter()
				.map(|(_, bill)| bill.amount)
				.sum::<f64>()
		} + {
			docker_repository_usages
				.iter()
				.map(|bill| bill.amount)
				.sum::<f64>()
		} + {
			domains_usages
				.iter()
				.map(|(_, bill)| bill.amount)
				.sum::<f64>()
		} + {
			secrets_usages
				.iter()
				.map(|(_, bill)| bill.amount)
				.sum::<f64>()
		};

		db::update_amount_due_for_workspace(
			&mut connection,
			&workspace.id,
			(existing_bill + new_bill).max(0f64),
		)
		.await?;
	}

	Ok(())
}
