use crate::{utils::settings::Settings, Database};

pub async fn migrate(
	_connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
	Ok(())
}
