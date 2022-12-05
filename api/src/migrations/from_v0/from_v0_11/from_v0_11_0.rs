use crate::{
	migrate_query as query,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	migrate_dollars_to_cents(connection, config).await?;
	Ok(())
}

pub(super) async fn migrate_dollars_to_cents(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	// transaction table migrations
	query!(
		r#"
		ALTER TABLE transaction
		RENAME COLUMN amount
		TO amount_in_cents;
		"#
	)
	.execute(&mut *connection)
	.await?;
	query!(
		r#"
		ALTER TABLE transaction
			ALTER COLUMN amount_in_cents TYPE BIGINT
				USING ROUND(amount_in_cents * 100)::BIGINT,
			ADD CONSTRAINT transaction_chk_amount_in_cents_positive
				CHECK(amount_in_cents >= 0);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// workspace table migrations
	query!(
		r#"
		ALTER TABLE workspace
		RENAME COLUMN amount_due
		TO amount_due_in_cents;
		"#
	)
	.execute(&mut *connection)
	.await?;
	query!(
		r#"
		ALTER TABLE workspace
			ALTER COLUMN amount_due_in_cents TYPE BIGINT
				USING ROUND(amount_due_in_cents * 100)::BIGINT,
			ADD CONSTRAINT workspace_chk_amount_due_in_cents_positive
				CHECK(amount_due_in_cents >= 0);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// coupon_code table migrations
	query!(
		r#"
		ALTER TABLE coupon_code
		RENAME COLUMN credits
		TO credits_in_cents;
		"#
	)
	.execute(&mut *connection)
	.await?;
	query!(
		r#"
		ALTER TABLE coupon_code
		ALTER COLUMN credits_in_cents TYPE BIGINT
		USING ROUND(credits_in_cents * 100)::BIGINT;
		"#
	)
	.execute(&mut *connection)
	.await?;
	query!(
		r#"
		ALTER TABLE coupon_code
		RENAME CONSTRAINT coupon_code_chk_credits_positive
		TO coupon_code_chk_credits_in_cents_positive;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
