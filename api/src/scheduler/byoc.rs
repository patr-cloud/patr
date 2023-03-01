use api_models::{models::workspace::region::RegionStatus, utils::Uuid};
use chrono::Utc;
use sqlx::Acquire;

use super::Job;
use crate::{db, service, utils::Error};

// Every day at 3 am
pub(super) fn check_status_of_active_byoc_regions_job() -> Job {
	Job::new(
		String::from("Update region connection status"),
		"0 0 3 * * *".parse().unwrap(),
		|| Box::pin(check_status_of_active_byoc_regions()),
	)
}

// Every day at 6 am
pub(super) fn handle_disconnected_byoc_regions_job() -> Job {
	Job::new(
		String::from("Handle disconnected byoc regions"),
		"0 0 6 * * *".parse().unwrap(),
		|| Box::pin(handle_disconnected_byoc_regions()),
	)
}

async fn check_status_of_active_byoc_regions() -> Result<(), Error> {
	let mut connection =
		super::CONFIG.get().unwrap().database.acquire().await?;

	let active_byoc_regions = db::get_all_active_byoc_region(&mut connection)
		.await?
		.into_iter()
		.filter_map(|region| {
			(region.status == RegionStatus::Active && region.is_byoc_region())
				.then_some((
					region.id,
					region.config_file?.0,
					region.ingress_hostname?,
				))
		});

	for (region_id, kubeconfig, prev_ingress_hostname) in active_byoc_regions {
		let mut connection = connection.begin().await?;

		let curr_ingress_hostname =
			service::get_patr_ingress_load_balancer_hostname(kubeconfig).await;

		match curr_ingress_hostname {
			Ok(Some(curr_ingress_hostname))
				if curr_ingress_hostname.to_string() ==
					prev_ingress_hostname =>
			{
				continue;
			}
			invalid_cases => {
				log::info!(
					"Invalid cases found while fetching status for region {} - {:?}",
					region_id,
					invalid_cases
				);
				log::info!(
					"So marking the cluster {region_id} as disconnected"
				);

				db::set_region_as_disconnected(
					&mut connection,
					&region_id,
					&Utc::now(),
				)
				.await?;
			}
		}

		connection.commit().await?;
	}

	Ok(())
}

async fn handle_disconnected_byoc_regions() -> Result<(), Error> {
	let mut connection =
		super::CONFIG.get().unwrap().database.acquire().await?;

	let disconnected_byoc_regions =
		db::get_all_disconnected_byoc_region(&mut connection)
			.await?
			.into_iter()
			.filter_map(|region| {
				(region.status == RegionStatus::Disconnected &&
					region.is_byoc_region())
				.then_some((
					region.id,
					region.workspace_id?,
					region.config_file?.0,
					region.ingress_hostname?,
					region.disconnected_at?,
				))
			});

	for (
		region_id,
		workspace_id,
		kubeconfig,
		prev_ingress_hostname,
		disconnected_at,
	) in disconnected_byoc_regions
	{
		let mut connection = connection.begin().await?;

		let curr_ingress_hostname =
			service::get_patr_ingress_load_balancer_hostname(kubeconfig).await;

		match curr_ingress_hostname {
			Ok(Some(curr_ingress_hostname))
				if curr_ingress_hostname.to_string() ==
					prev_ingress_hostname =>
			{
				log::info!(
					"Region `{}` got connected again. So marking it as active",
					region_id
				);
				db::set_region_as_connected(
					&mut connection,
					&region_id,
				)
				.await?;
			}
			invalid_cases => {
				log::info!(
					"Invalid cases found while fetching status for region {} - {:?}",
					region_id,
					invalid_cases
				);

				let disconnected_at = Utc::now()
					.signed_duration_since(disconnected_at)
					.num_days() as u64;

				if disconnected_at > 7 {
					// mark all the deployments for that region as deleted and
					// also mark the region as deleted.
					let app_config = super::CONFIG.get().unwrap();

					let request_id = Uuid::new_v4();
					log::info!(
						"request_id {} - Deleting deployments for region {} as it is not connected",
						request_id,
						region_id
					);

					let deployments = db::get_deployments_by_region_id(
						&mut connection,
						&workspace_id,
						&region_id,
					)
					.await?;

					for deployment in &deployments {
						service::delete_deployment(
							&mut connection,
							&workspace_id,
							&deployment.id,
							&region_id,
							None,
							None,
							"0.0.0.0",
							true,
							false,
							&app_config.config,
							&request_id,
						)
						.await?
					}

					db::delete_region(&mut connection, &region_id, &Utc::now())
						.await?;
				} else {
					service::send_byoc_region_disconnected_reminder_email(
						&mut connection,
						&workspace_id,
						&region_id,
						7 - disconnected_at,
					)
					.await?;
				}
			}
		}

		connection.commit().await?;
	}

	Ok(())
}
