use std::cmp::{max, min};

use api_models::utils::DateTime;
use chrono::{Datelike, TimeZone, Utc};

use super::Job;
use crate::{
	db,
	models::{deployment, UserDeployment},
	service,
	utils::Error,
};

// Every two hours
pub(super) fn generate_report_card_job() -> Job {
	Job::new(
		String::from("Update bills of workspaces"),
		"0 0 */15 * * *".parse().unwrap(),
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
			db::get_deployments_for_report_card(&mut connection, &workspace.id)
				.await?;

		let mut user_deployment = Vec::new();
		for deployment in deployments {
			let machine_type = db::get_machine_type_id(
				&mut connection,
				&deployment.machine_type,
			)
			.await?;

			let deployment_usages = db::get_deployment_usage(
				&mut *connection,
				&deployment.id,
				&DateTime::from(month_start_date),
				&Utc::now(),
			)
			.await?;

			let mut hours = 0;
			let mut monthly_price = 0f32;
			let mut cpu_count = 0;
			let mut memory_count = 0;
			let remaining_free_hours = 720;
			let price_in_cents = 0;
			let mut plan = String::new();
			let mut estimated_cost = 0.0;
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

				let (cpu, memory) = deployment::MACHINE_TYPES
					.get()
					.unwrap()
					.get(&deployment_usage.machine_type)
					.unwrap_or(&(1, 2));

				cpu_count = *cpu as u32;
				memory_count = *memory as u32;

				monthly_price = match (cpu_count, memory_count) {
					(1, 2) => 5f32,
					(1, 4) => 10f32,
					(1, 8) => 20f32,
					(2, 8) => 40f32,
					(4, 32) => 80f32,
					_ => 0f32,
				};

				if (cpu_count, memory_count) == (1, 2) &&
					deployment_usage.num_instance == 1
				{
					// Free eligible
					plan = "Free".to_string();
					estimated_cost = 0.0 as f32;
				} else {
					plan = monthly_price.to_string();
					estimated_cost = monthly_price as f32 *
						deployment_usage.num_instance as f32;
				}

				break;
			}

			user_deployment.push(UserDeployment {
				deployment_id: deployment.id,
				deployment_name: deployment.name,
				hours,
				instances: deployment.max_horizontal_scale as u64,
				estimated_cost,
				ram_count: machine_type.memory_count as u32,
				cpu_count: machine_type.cpu_count as u32,
				plan,
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
