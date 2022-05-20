use api_macros::query;

use crate::{
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE workspace
			ADD COLUMN alert_emails VARCHAR(320) [] NOT NULL 
			DEFAULT ARRAY[]::VARCHAR[];
		"#,
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace
			ALTER COLUMN alert_emails DROP DEFAULT;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE workspace w1
		SET alert_emails = (
			SELECT 
				ARRAY_AGG(CONCAT("user".recovery_email_local, '@', domain.name, '.', domain.tld))
			FROM 
				workspace w2
			INNER JOIN
				"user"
			ON
				"user".id = w2.super_admin_id
			INNER JOIN
				domain
			ON
				"user".recovery_email_domain_id = domain.id
			WHERE
				w2.id = w1.id
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
