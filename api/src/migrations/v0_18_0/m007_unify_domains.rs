//! Unify domain handling — drops the Patr-controlled vs user-controlled split.
//!
//! Self-hosted mode has no concept of a Patr-managed nameserver, so the
//! distinction collapses. All `workspace_domain` rows become regular,
//! externally-managed entries.
//!
//! - Drops `patr_domain_dns_record` (and its FK to `patr_controlled_domain`)
//! - Drops `patr_controlled_domain` and `user_controlled_domain`
//! - Drops the `workspace_domain_uq_id_nameserver_type` constraint and the
//!   `workspace_domain.nameserver_type` column
//! - Drops the `DNS_RECORD_TYPE` and `DOMAIN_NAMESERVER_TYPE` enums
//!
//! Orphaned `resource` rows for the dropped DNS records are intentionally
//! left in place — `audit_log.resource_id` references them and cleaning the
//! audit trail is out of scope.

use crate::prelude::*;

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		DROP TABLE IF EXISTS patr_domain_dns_record;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DROP TABLE IF EXISTS patr_controlled_domain;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DROP TABLE IF EXISTS user_controlled_domain;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE workspace_domain
			DROP CONSTRAINT IF EXISTS workspace_domain_uq_id_nameserver_type,
			DROP COLUMN IF EXISTS nameserver_type;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DROP TYPE IF EXISTS DNS_RECORD_TYPE;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DROP TYPE IF EXISTS DOMAIN_NAMESERVER_TYPE;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
