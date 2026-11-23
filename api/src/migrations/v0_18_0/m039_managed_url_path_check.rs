//! Limits a managed URL's path to characters that can't break out of the
//! Caddyfile site address the runner writes it into. The API has validated
//! paths against the same pattern since this change, and the check keeps the
//! database to it too.
//!
//! Managed URLs whose path already breaks the rule are logged by ID first, and
//! then adding the check fails.

use crate::prelude::*;

#[macros::migration]
async fn migrate(
	connection: &mut DatabaseConnection,
	_config: &AppConfig,
) -> Result<(), ErrorType> {
	let violators = sqlx::query_scalar::<_, Uuid>(
		r#"
		SELECT
			id
		FROM
			managed_url
		WHERE
			path !~ '^/([a-z0-9._~-]+(/[a-z0-9._~-]+)*/?)?$';
		"#,
	)
	.fetch_all(&mut *connection)
	.await?;

	if !violators.is_empty() {
		error!(
			"Managed URLs with a path that isn't allowed: {}",
			violators
				.iter()
				.map(ToString::to_string)
				.collect::<Vec<_>>()
				.join(", ")
		);
	}

	sqlx::query(
		r#"
		ALTER TABLE managed_url
			ADD CONSTRAINT managed_url_chk_path_valid CHECK(
				path ~ '^/([a-z0-9._~-]+(/[a-z0-9._~-]+)*/?)?$'
			);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
