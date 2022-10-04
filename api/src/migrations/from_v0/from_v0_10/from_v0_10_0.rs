use crate::{
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	fix_timestamps(connection, config).await?;

	Ok(())
}

async fn fix_timestamps(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		UPDATE
			resource
		SET
			created = TO_TIMESTAMP(EXTRACT(EPOCH FROM created) / 1000)
		WHERE
			created > '4000-01-01';
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			"user"
		SET
			created = TO_TIMESTAMP(EXTRACT(EPOCH FROM created) / 1000)
		WHERE
			created > '4000-01-01';
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			"user"
		SET
			dob = TO_TIMESTAMP(EXTRACT(EPOCH FROM dob) / 1000)
		WHERE
			dob IS NOT NULL AND
			dob > '4000-01-01';
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			user_to_sign_up
		SET
			otp_expiry = TO_TIMESTAMP(EXTRACT(EPOCH FROM otp_expiry) / 1000)
		WHERE
			otp_expiry > '4000-01-01';
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			user_unverified_personal_email
		SET
			verification_token_expiry = TO_TIMESTAMP(
				EXTRACT(EPOCH FROM verification_token_expiry) / 1000
			)
		WHERE
			verification_token_expiry > '4000-01-01';
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			user_unverified_phone_number
		SET
			verification_token_expiry = TO_TIMESTAMP(
				EXTRACT(EPOCH FROM verification_token_expiry) / 1000
			)
		WHERE
			verification_token_expiry > '4000-01-01';
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			user_login
		SET
			token_expiry = TO_TIMESTAMP(
				EXTRACT(EPOCH FROM token_expiry) / 1000
			)
		WHERE
			token_expiry > '4000-01-01';
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			user_login
		SET
			last_login = TO_TIMESTAMP(
				EXTRACT(EPOCH FROM last_login) / 1000
			)
		WHERE
			last_login > '4000-01-01';
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			user_login
		SET
			last_activity = TO_TIMESTAMP(
				EXTRACT(EPOCH FROM last_activity) / 1000
			)
		WHERE
			last_activity > '4000-01-01';
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			password_reset_request
		SET
			token_expiry = TO_TIMESTAMP(
				EXTRACT(EPOCH FROM token_expiry) / 1000
			)
		WHERE
			token_expiry > '4000-01-01';
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			docker_registry_repository_manifest
		SET
			created = TO_TIMESTAMP(
				EXTRACT(EPOCH FROM created) / 1000
			)
		WHERE
			created > '4000-01-01';
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			docker_registry_repository_tag
		SET
			last_updated = TO_TIMESTAMP(
				EXTRACT(EPOCH FROM last_updated) / 1000
			)
		WHERE
			last_updated > '4000-01-01';
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
