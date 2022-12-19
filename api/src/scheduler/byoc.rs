use chrono::Utc;

use super::Job;
use crate::{db, service, utils::Error};

// Every day at 4 am
pub(super) fn recheck_connection_to_cluster_job() -> Job {
	Job::new(
		String::from("Update bills of workspaces"),
		"0 0 4 * * *".parse().unwrap(),
		|| Box::pin(recheck_connection_to_cluster()),
	)
}

async fn recheck_connection_to_cluster() -> Result<(), Error> {
	let mut connection =
		super::CONFIG.get().unwrap().database.acquire().await?;
	let byoc_regions = db::get_all_ready_byoc_region(&mut connection).await?;

	for region in byoc_regions {
		let ip_addr = service::get_external_ip_addr_for_load_balancer(
			"ingress-nginx",
			"ingress-nginx-controller",
			&region.config_file,
		)
		.await?;

		if ip_addr.is_some() {
			let deployment_region =
				db::get_region_by_id(&mut connection, &region.id).await?;
			if let Some(deployment_region) = deployment_region {
				if deployment_region.last_disconnected.is_some() {
					db::update_byoc_region_connected(
						&mut connection,
						&region.id,
						None,
					)
					.await?;
				}
			}
		} else if let Some(last_disconnected) = region.last_disconnected {
			let last_disconnected = Utc::now()
				.signed_duration_since(last_disconnected)
				.num_days();
			if last_disconnected > 7 {
				// Mark all deployment for that region as deleted
				let deployments = db::get_deployments_by_region_id(
					&mut connection,
					&region.workspace_id,
					&region.id,
				)
				.await?;

				for deployment in deployments {
					db::delete_deployment(
						&mut connection,
						&deployment.id,
						&Utc::now(),
					)
					.await?;
				}

				// Delete region
				db::delete_region(&mut connection, &region.id, &Utc::now())
					.await?;

			// TODO - Send a notification email about the event
			} else {
				// TODO - Send a reminder email about you cluster is not
				// connected
				continue;
			}
		} else {
			db::update_byoc_region_connected(
				&mut connection,
				&region.id,
				Some(&Utc::now()),
			)
			.await?;

			// TODO - Send a Reminder email
		}
	}

	Ok(())
}
