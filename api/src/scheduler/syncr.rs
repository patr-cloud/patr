use api_models::utils::Uuid;

use super::Job;
use crate::{db, service, syncr, utils::Error};

// every day at 6 AM
pub(super) fn initiate_syncr_job() -> Job {
	Job::new(
		String::from("Initiate syncr to sync db with k8s"),
		"0 0 6 * * *".parse().unwrap(),
		|| Box::pin(initiate_syncr()),
	)
}

pub async fn initiate_syncr() -> Result<(), Error> {
	let app = super::CONFIG
		.get()
		.expect("CONFIG must be initialized before calling this function");
	let mut connection = app.database.acquire().await?;
	let kube_client = service::get_kubernetes_config(&app.config).await?;

	let workspaces = db::get_all_workspaces(&mut connection).await?;
	for workspace in workspaces {
		let mut connection =
			super::CONFIG.get().unwrap().database.begin().await?;

		let request_id = Uuid::new_v4();
		log::info!(
			"request_id: {} - Syncing resources for workspace {}",
			request_id,
			workspace.id
		);

		syncr::sync_deployments_in_workspace(
			&workspace.id,
			&mut *connection,
			kube_client.clone(),
			&app.config,
			&request_id,
		)
		.await?;

		connection.commit().await?;
	}

	Ok(())
}
