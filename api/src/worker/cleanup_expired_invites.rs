use apalis::prelude::*;
use apalis_cron::Tick;
use time::OffsetDateTime;

use crate::prelude::*;

/// The cron job that deletes workspace invites that expired more than
/// [`WORKSPACE_INVITE_RETENTION`][1] ago.
///
/// [1]: constants::WORKSPACE_INVITE_RETENTION
pub async fn cleanup_expired_invites(_: Tick, data: Data<AppState>) -> Result<(), WorkerError> {
	info!("Cleaning up long-expired workspace invites...");

	let cutoff = OffsetDateTime::now_utc() - constants::WORKSPACE_INVITE_RETENTION;

	// Both deletes run in one transaction: a concurrent resend can bump an
	// invite's `token_expiry` between the two statements, which would leave
	// a live invite stripped of its role rows.
	let mut transaction = data
		.database
		.begin()
		.await
		.map_err(|err| WorkerStateError::InvalidState(err.to_string()))?;

	// The role rows go first — the FK to `workspace_user_invite` is not
	// `ON DELETE CASCADE`.
	query!(
		r#"
		DELETE FROM
			workspace_user_invite_role
		WHERE
			invite_id IN (
				SELECT
					id
				FROM
					workspace_user_invite
				WHERE
					token_expiry < $1
			);
		"#,
		cutoff,
	)
	.execute(&mut *transaction)
	.await
	.map_err(|err| WorkerStateError::InvalidState(err.to_string()))?;

	let deleted = query!(
		r#"
		DELETE FROM
			workspace_user_invite
		WHERE
			token_expiry < $1;
		"#,
		cutoff,
	)
	.execute(&mut *transaction)
	.await
	.map_err(|err| WorkerStateError::InvalidState(err.to_string()))?
	.rows_affected();

	transaction
		.commit()
		.await
		.map_err(|err| WorkerStateError::InvalidState(err.to_string()))?;

	info!("Deleted {deleted} long-expired invite(s)");

	Ok(())
}
