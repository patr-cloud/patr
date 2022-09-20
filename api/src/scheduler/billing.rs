use api_models::utils::Uuid;
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

// Runs everyday at 8am
pub(super) fn verify_resource_usage_payment_monthly_job() -> Job {
	Job::new(
		String::from("Check payment for monthly resource usage"),
		"0 0 8 * * *".parse().unwrap(),
		|| Box::pin(verify_resource_usage_payment_monthly()),
	)
}

async fn update_bill() -> Result<(), Error> {
	let mut connection =
		super::CONFIG.get().unwrap().database.acquire().await?;
	let workspaces = db::get_all_workspaces(&mut connection).await?;
	let now = Utc::now();
	let month_start_date = now.date().with_day(1).unwrap().and_hms(0, 0, 0);

	for workspace in workspaces {
		let mut connection =
			super::CONFIG.get().unwrap().database.begin().await?;
		let new_bill = service::calculate_total_bill_for_workspace_till(
			&mut connection,
			&workspace.id,
			&month_start_date,
			&now,
		)
		.await?;

		db::update_amount_due_for_workspace(
			&mut connection,
			&workspace.id,
			new_bill,
		)
		.await?;
		connection.commit().await?;
	}

	Ok(())
}

async fn verify_resource_usage_payment_monthly() -> Result<(), Error> {
	let mut connection =
		super::CONFIG.get().unwrap().database.acquire().await?;

	let request_id = Uuid::new_v4();
	let config = super::CONFIG.get().unwrap().config.clone();

	// Get all workspaces
	let workspaces = db::get_all_workspaces(&mut connection).await?;
	let now = Utc::now();
	let current_month_day = now.day();

	for workspace in workspaces {
		let mut connection =
			super::CONFIG.get().unwrap().database.begin().await?;

		let total_amount =
			db::get_total_bill(&mut connection, &workspace.id).await?;

		if total_amount > 0.0 {
			if current_month_day < 15 {
				// sent reminder mail for payment daily for 15 days
				todo!()
			} else {
				// delete all resources
				service::delete_all_resources_in_workspace(
					&mut connection,
					&workspace.id,
					&workspace.super_admin_id,
					&config,
					&request_id,
				)
				.await?;

				// Reset resource limit to zero
				db::reset_resource_limit_on_workspace(
					&mut connection,
					&workspace.id,
				)
				.await?;

				connection.commit().await?;
				// and send a mail
				todo!()
			}
		} else {
			continue;
		}
	}

	Ok(())
}
