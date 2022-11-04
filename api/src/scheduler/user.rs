use super::Job;
use crate::{db, utils::Error};

// Every hour
pub(super) fn revoke_expired_tokens_job() -> Job {
	Job::new(
		String::from("Revoke all expired user API tokens"),
		"0 0 * * * *".parse().unwrap(),
		|| Box::pin(revoke_expired_tokens()),
	)
}

async fn revoke_expired_tokens() -> Result<(), Error> {
	let mut connection =
		super::CONFIG.get().unwrap().database.acquire().await?;

	db::revoke_all_expired_user_tokens(&mut connection).await?;

	Ok(())
}
