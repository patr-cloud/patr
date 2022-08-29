use crate::{
	migrate_query as query,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	create_static_site_upload_history(&mut *connection, config).await?;

	Ok(())
}

async fn create_static_site_upload_history(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE coupon_code
		DROP CONSTRAINT coupon_code_chk_credits_positive;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE coupon_code
		ADD CONSTRAINT coupon_code_chk_credits_positive CHECK(credits >= 0);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
