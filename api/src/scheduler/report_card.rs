use std::cmp::{max, min};

use api_models::utils::DateTime;
use chrono::{Datelike, Local, TimeZone, Utc, Weekday};

use super::Job;
use crate::{
	db,
	models::{deployment, UserDeployment},
	service,
	utils::Error,
};

// Every day at 6
pub(super) fn generate_report_card_job() -> Job {
	Job::new(
		String::from("Update bills of workspaces"),
		"0 0 6 * * *".parse().unwrap(),
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

	let today = Local::now();
	let week_of_month = (today.day() - 1) / 7 + 1;
	if (week_of_month == 2 || week_of_month == 4) &&
		now.weekday() == Weekday::Mon
	{
		for workspace in workspaces {
			let mut connection =
				super::CONFIG.get().unwrap().database.begin().await?;
			let deployments = db::get_deployments_for_report_card(
				&mut connection,
				&workspace.id,
			)
			.await?;

			let mut user_deployment = Vec::new();
			for deployment in &deployments {
				let deployment_usages = db::get_deployment_usage(
					&mut connection,
					&deployment.id,
					&DateTime::from(month_start_date),
					&Utc::now(),
				)
				.await?;

				let mut hours = 0;
				let mut cpu_count = 0;
				let mut memory_count = 0;
				let mut plan = "Free".to_string();
				let mut estimated_cost = 0;
				let mut instances = 1u32;

				let deployment_report = deployment_usages
					.into_iter()
					.next()
					.map(|deployment_usage| {
						let stop_time = deployment_usage
							.stop_time
							.map(chrono::DateTime::from)
							.unwrap_or_else(Utc::now);
						let start_time =
							max(deployment_usage.start_time, month_start_date);
						hours = min(
							720,
							((stop_time - start_time).num_seconds() as f64 /
								3600f64)
								.ceil() as i64,
						) as u64;

						let (cpu, memory) = deployment::MACHINE_TYPES
							.get()
							.unwrap()
							.get(&deployment_usage.machine_type)
							.unwrap_or(&(1, 2));

						cpu_count = *cpu as u32;
						memory_count = *memory as u32;

						let monthly_price = match (cpu_count, memory_count) {
							(1, 2) => 5u32,
							(1, 4) => 10u32,
							(1, 8) => 20u32,
							(2, 8) => 40u32,
							(4, 32) => 80u32,
							_ => 0u32,
						};

						if (cpu_count, memory_count) == (1, 2) &&
							deployment_usage.num_instance == 1
						{
							// Free eligible
							plan = "Free".to_string();
							estimated_cost = 0;
						} else {
							plan = monthly_price.to_string();
							estimated_cost = monthly_price *
								deployment_usage.num_instance as u32;
						}

						instances = deployment_usage.num_instance as u32;

						UserDeployment {
							deployment_id: deployment.id.clone(),
							deployment_name: deployment.name.clone(),
							hours,
							instances,
							estimated_cost: estimated_cost * 100,
							ram_count: memory_count,
							cpu_count,
							plan: plan.clone(),
						}
					})
					.unwrap_or(UserDeployment {
						deployment_id: deployment.id.clone(),
						deployment_name: deployment.name.clone(),
						hours,
						instances,
						estimated_cost: estimated_cost * 100,
						ram_count: memory_count,
						cpu_count,
						plan: plan.clone(),
					});

				user_deployment.push(deployment_report)
			}

			service::send_report_card_email_notification(
				&mut connection,
				&workspace,
				"Deployment", // Resource Type
				&user_deployment,
			)
			.await?;
		}
	}

	Ok(())
}
