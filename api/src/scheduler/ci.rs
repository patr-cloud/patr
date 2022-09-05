use api_models::utils::Uuid;

use super::Job;
use crate::{db, service, utils::Error};

// Every day at 6 AM
pub(super) fn sync_repo_job() -> Job {
	Job::new(
		String::from("Sync repo for CI"),
		"0 0 6 * * *".parse().unwrap(),
		|| Box::pin(sync_repos()),
	)
}

async fn sync_repos() -> Result<(), Error> {
	let mut connection =
		super::CONFIG.get().unwrap().database.acquire().await?;
	let workspaces = db::get_all_workspaces(&mut connection).await?;

	for workspace in workspaces {
		let request_id = Uuid::new_v4();

		let mut connection =
			super::CONFIG.get().unwrap().database.begin().await?;

		let connected_git_providers =
			db::list_connected_git_providers_for_workspace(
				&mut connection,
				&workspace.id,
			)
			.await?;

		for git_provider in connected_git_providers {
			log::info!("request_id: {} - Syncing repos for workspace {} from git_provider {}", request_id, workspace.id, git_provider.id);
			service::sync_repos_for_git_provider(
				&mut connection,
				&git_provider,
				&request_id,
			)
			.await?;
		}

		connection.commit().await?;
	}

	Ok(())
}
