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
			ADD COLUMN alert_emails VARCHAR(320) [];
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
