use apalis::prelude::*;
use apalis_cron::Tick;
use futures::TryStreamExt;

use crate::prelude::*;

/// The cron job that verifies unverified domains every 2 hours.
///
/// For every unverified domain in the database, get it's Cloudflare custom
/// hostname ID, and hit the cloudflare endpoint to check if it's verified. If
/// it is, update the database to mark it as verified.
pub async fn verify_unverified_domains(_: Tick, data: Data<AppState>) -> Result<(), WorkerError> {
	_ = query!(
		r#"
        SELECT
            id,
            cloudflare_custom_hostname_id
        FROM
            workspace_domain
        WHERE
            is_verified = FALSE;
        "#,
	)
	.fetch(&data.database)
	.try_for_each(async |domain| {
		// Check with Cloudflare if the domain is verified
		Ok(())
	})
	.await;

	Ok(())
}
