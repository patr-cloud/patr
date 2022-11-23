use chrono::{Datelike, TimeZone, Utc};

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
	let mut connection =
		super::CONFIG.get().unwrap().database.acquire().await?;
	let workspaces = db::get_all_workspaces(&mut connection).await?;
	let now = Utc::now();
	let month_start_date = Utc
		.with_ymd_and_hms(now.year(), now.month(), 1, 0, 0, 0)
		.unwrap();

	for workspace in workspaces {
		let mut connection =
			super::CONFIG.get().unwrap().database.begin().await?;
		let total_resource_usage_bill =
			service::calculate_total_bill_for_workspace_till(
				&mut connection,
				&workspace.id,
				&month_start_date,
				&now,
			)
			.await?;

		db::update_amount_due_for_workspace(
			&mut connection,
			&workspace.id,
			total_resource_usage_bill.total_charge,
		)
		.await?;
		connection.commit().await?;
	}

	Ok(())
}
