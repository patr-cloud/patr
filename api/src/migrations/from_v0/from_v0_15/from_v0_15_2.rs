use crate::{
	migrate_query as query,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	add_referred_from_for_user(connection, config).await?;
	add_referred_from_for_user_to_sign_up(connection, config).await?;
	add_is_referral_for_coupon(connection, config).await?;
	add_last_referred_for_user(connection, config).await?;
	add_referral_click_for_user(connection, config).await?;

	Ok(())
}

async fn add_referred_from_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE "user" 
		ADD COLUMN referred_from TEXT;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn add_referred_from_for_user_to_sign_up(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE user_to_sign_up 
		ADD COLUMN referred_from TEXT;
		"#
	)
	.execute(&mut *connection)
	.await?;

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

async fn add_referral_click_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE "user"
		ADD COLUMN referral_click INTEGER NOT NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
