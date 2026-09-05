//! Repoints the audit log's actor reference at the `actor_client` registry.
//!
//! `audit_log.login_id` answered "who did this" only for user logins. With
//! `actor_client` as the credential supertype (and `login_id` values reused
//! as its ids), renaming the column and swapping the FK makes the audit log
//! credential-kind-agnostic without moving any data: every stored value is
//! already a valid `actor_client` id.

use crate::prelude::*;

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		ALTER TABLE audit_log RENAME COLUMN login_id TO actor_client_id;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE audit_log DROP CONSTRAINT audit_log_login_id_fkey;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE audit_log
		ADD CONSTRAINT audit_log_fk_actor_client_id
		FOREIGN KEY(actor_client_id) REFERENCES actor_client(id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
