use crate::{
	migrate_query as query,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		CREATE TABLE coupon_code(
			code TEXT CONSTRAINT coupon_code_pk PRIMARY KEY,
			credits INTEGER NOT NULL CONSTRAINT coupon_code_chk_credits_positive
				CHECK(credits > 0),
			expiry TIMESTAMPTZ,
			uses_remaining INTEGER CONSTRAINT
				coupon_code_chk_uses_remaining_positive CHECK(
					uses_remaining IS NULL OR uses_remaining > 0
				)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE "user"
		ADD COLUMN sign_up_coupon TEXT
		CONSTRAINT user_fk_sign_up_coupon REFERENCES coupon_code(code);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_to_sign_up
		ADD COLUMN coupon_code TEXT
		CONSTRAINT user_to_sign_up_fk_coupon_code REFERENCES coupon_code(code);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
