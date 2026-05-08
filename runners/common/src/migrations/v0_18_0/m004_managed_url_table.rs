//! Add the `managed_url` table for runner-side managed URL tracking.
//!
//! Stores resolved hosts so the Caddy ingress writer doesn't need to look up
//! domain info at config-write time.

use crate::prelude::*;

/// Create the `managed_url` table and its index.
#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), sqlx::Error> {
	query(
		r#"
		CREATE TABLE managed_url(
			id TEXT NOT NULL PRIMARY KEY,
			host TEXT NOT NULL,
			path TEXT NOT NULL,
			deployment_id TEXT NOT NULL,
			port INTEGER NOT NULL,

			CONSTRAINT managed_url_chk_port_range
				CHECK(port > 0 AND port <= 65535),
			CONSTRAINT managed_url_chk_host_nonempty
				CHECK(LENGTH(TRIM(host)) > 0),

			FOREIGN KEY(deployment_id) REFERENCES deployment(id)
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	query(
		r#"
		CREATE INDEX
			managed_url_idx_deployment_id
		ON
			managed_url(deployment_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
