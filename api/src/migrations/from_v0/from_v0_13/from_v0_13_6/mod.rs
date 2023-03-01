mod cf_byoc;

use crate::{
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	cf_byoc::migrate(connection, config).await?;
	Ok(())
}
