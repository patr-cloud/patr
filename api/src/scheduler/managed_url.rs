use api_models::{self, utils::Uuid};

use crate::{scheduler::Job, utils::Error};

// Every two hours
pub(super) fn verify_unverified_managed_url_job() -> Job {
	Job::new(
		String::from("Verify unverified domains"),
		"0 0 4 * * *".parse().unwrap(),
		|| Box::pin(verify_unverified_managed_url()),
	)
}

async fn verify_unverified_managed_url() -> Result<(), Error> {
	let request_id = Uuid::new_v4();
	log::trace!(
		"request_id: {} - Verifying unverified managed url",
		request_id
	);
	let config = super::CONFIG.get().unwrap();
	let _connection = config.database.acquire().await?;

	// TODO - Logic to verify unverified managed url

	Ok(())
}
