mod cloudflare_ingress;

use crate::{
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	cloudflare_ingress::migrate(connection, config).await?;
	Ok(())
}
