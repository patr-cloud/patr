use crate::{
	migrate_query as query,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
<<<<<<< HEAD
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

=======
	config: &Settings,
) -> Result<(), Error> {
	add_table_deployment_image_digest(&mut *connection, config).await?;
>>>>>>> feature: deployment revert to specific image sha
	Ok(())
}

async fn add_table_deployment_image_digest(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		CREATE TABLE deployment_deploy_history(
			deployment_id UUID NOT NULL
				CONSTRAINT deployment_image_digest_fk_deployment_id
					REFERENCES deployment(id),
			image_digest TEXT NOT NULL,
			repository_id UUID NOT NULL
				CONSTRAINT deployment_image_digest_fk_repository_id
					REFERENCES docker_registry_repository(id),
			message TEXT,
			created BIGINT NOT NULL
				CONSTRAINT deployment_deploy_history_chk_created_unsigned CHECK(
						created >= 0
				),
			CONSTRAINT deployment_image_digest_pk
				PRIMARY KEY(deployment_id, image_digest)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;
	Ok(())
}