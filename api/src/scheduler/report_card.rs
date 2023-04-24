use std::cmp::{max, min};

use api_models::utils::DateTime;
use chrono::{Datelike, TimeZone, Utc};

use super::Job;
use crate::{db, models::UserDeployment, service, utils::Error};

// Every two hours
pub(super) fn generate_report_card_job() -> Job {
	Job::new(
		String::from("Update bills of workspaces"),
		"0 0 1/2 * * *".parse().unwrap(),
		|| Box::pin(generate_report_card()),
	)
}

async fn generate_report_card() -> Result<(), Error> {
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
		let deployments =
			db::get_deployments_for_workspace(&mut connection, &workspace.id)
				.await?;

		let user_deployment = Vec::new();
		for deployment in deployments {
			let machint_type = db::get_machine_type_id(
				&mut connection,
				&deployment.machine_type,
			)
			.await?;

			let deployment_usages = db::get_all_deployment_usage(
				&mut *connection,
				&workspace.id,
				&DateTime::from(month_start_date),
				&Utc::now(),
			)
			.await?;

			let mut hours = 0;
			for deployment_usage in deployment_usages {
				let stop_time = deployment_usage
					.stop_time
					.map(chrono::DateTime::from)
					.unwrap_or_else(|| Utc::now());
				let start_time =
					max(deployment_usage.start_time, month_start_date);
				hours = min(
					720,
					((stop_time - start_time).num_seconds() as f64 / 3600f64)
						.ceil() as i64,
				) as u64;
			}

			// TODO get plan based on deployment
			let plan = todo!();

			// TODO - estimated cost
			let estimatedCost = todo!();

			user_deployment.push(UserDeployment {
				deployment_id: deployment.id,
				deployment_name: deployment.name,
				hours,
				instances: deployment.max_horizontal_scale as u64,
				estimated_cost: 15.0, // TODO - Change this after calculation
				ram_count: machint_type.memory_count as u32,
				cpu_count: machint_type.cpu_count as u32,
				plan: "Free".to_string(), /* TODO -  change  plan after
				                           * calculation */
			})
		}

		service::send_report_card_email_notification(
			&mut connection,
			&workspace,
			"Deployment", // Resource Type
			&user_deployment,
		)
		.await?;
	}

	Ok(())
}
