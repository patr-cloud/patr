use super::Job;
use crate::utils::Error;

// Every two hours
pub(super) fn update_bill_job() -> Job {
	Job::new(
		String::from("Update bills of workspaces"),
		"0 0 1/2 * * *".parse().unwrap(),
		|| Box::pin(update_bill()),
	)
}

async fn update_bill() -> Result<(), Error> {
	Ok(())
}
