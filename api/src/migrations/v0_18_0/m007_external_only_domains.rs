//! Drops Patr-controlled (internal-DNS) domain support entirely.
//!
//! Removes:
//! - `patr_domain_dns_record` table
//! - `patr_controlled_domain` table
//! - `user_controlled_domain` table (its only purpose was the discriminator FK)
//! - `workspace_domain.nameserver_type` column
//! - `DOMAIN_NAMESERVER_TYPE` enum
//! - `DNS_RECORD_TYPE` enum

use crate::prelude::*;

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	for stmt in [
		"DROP TABLE IF EXISTS patr_domain_dns_record CASCADE",
		"DROP TABLE IF EXISTS patr_controlled_domain CASCADE",
		"DROP TABLE IF EXISTS user_controlled_domain CASCADE",
		"DROP TYPE IF EXISTS DNS_RECORD_TYPE CASCADE",
	] {
		sqlx::query(stmt).execute(&mut *connection).await?;
	}

	sqlx::query(
		r#"
		ALTER TABLE workspace_domain
			DROP CONSTRAINT IF EXISTS workspace_domain_uq_id_nameserver_type;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE workspace_domain
			DROP COLUMN IF EXISTS nameserver_type;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query("DROP TYPE IF EXISTS DOMAIN_NAMESERVER_TYPE CASCADE;")
		.execute(&mut *connection)
		.await?;

	Ok(())
}
