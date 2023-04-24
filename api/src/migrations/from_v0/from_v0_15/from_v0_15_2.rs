use crate::{
	migrate_query as query,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	add_is_referral_for_coupon(connection, config).await?;
	add_last_referred_for_user(connection, config).await?;

	Ok(())
}

async fn add_is_referral_for_coupon(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE coupon_code 
		ADD COLUMN is_referral BOOL NOT NULL DEFAULT false;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn add_last_referred_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE "user"
		ADD COLUMN last_referred TIMESTAMPTZ;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
