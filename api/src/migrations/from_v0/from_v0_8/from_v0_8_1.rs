use crate::{
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn migrate(
	_connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	Ok(())
}
