use crate::utils::Error;

use super::Job;

// Every two hours
// TODO: change this to one hours
pub(super) fn update_bill() -> Job {
	Job::new(
		String::from("Verify unverified domains"),
		"0 0 1/2 * * *".parse().unwrap(),
		|| Box::pin(update_bill()),
	)
}

// TODO: fill this
pub async fn update_bill() -> Result<(), Error> {
   Ok(())
}