use chrono::{Duration, Utc};
use redis::{AsyncCommands, Client};

use crate::{
	migrate_query as query,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	migrate_from_bigint_to_timestamptz(connection, config).await?;
	create_user_transferring_domain_to_patr_table(&mut *connection, config)
		.await?;
	Ok(())
}

async fn migrate_from_bigint_to_timestamptz(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE resource
		DROP CONSTRAINT resource_created_chk_unsigned;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE resource
		ALTER COLUMN created TYPE TIMESTAMPTZ
		USING TO_TIMESTAMP(created);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE "user"
		DROP CONSTRAINT user_chk_created_unsigned;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE "user"
		ALTER COLUMN created TYPE TIMESTAMPTZ
		USING TO_TIMESTAMP(created);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE "user"
		DROP CONSTRAINT user_chk_dob_unsigned;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE "user"
		ALTER COLUMN dob TYPE TIMESTAMPTZ
		USING TO_TIMESTAMP(dob);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_to_sign_up
		DROP CONSTRAINT user_to_sign_up_chk_expiry_unsigned;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_to_sign_up
		ALTER COLUMN otp_expiry TYPE TIMESTAMPTZ
		USING TO_TIMESTAMP(otp_expiry);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_unverified_personal_email
		DROP CONSTRAINT user_unverified_personal_email_chk_token_expiry_unsigned;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_unverified_personal_email
		ALTER COLUMN verification_token_expiry TYPE TIMESTAMPTZ
		USING TO_TIMESTAMP(verification_token_expiry);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_unverified_phone_number
		DROP CONSTRAINT user_unverified_phone_number_chk_token_expiry_unsigned;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_unverified_phone_number
		ALTER COLUMN verification_token_expiry TYPE TIMESTAMPTZ
		USING TO_TIMESTAMP(verification_token_expiry);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_login
		DROP CONSTRAINT user_login_chk_token_expiry_unsigned;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_login
		ALTER COLUMN token_expiry TYPE TIMESTAMPTZ
		USING TO_TIMESTAMP(token_expiry);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_login
		DROP CONSTRAINT user_login_chk_last_login_unsigned;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_login
		ALTER COLUMN last_login TYPE TIMESTAMPTZ
		USING TO_TIMESTAMP(last_login);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_login
		DROP CONSTRAINT user_login_chk_last_activity_unsigned;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_login
		ALTER COLUMN last_activity TYPE TIMESTAMPTZ
		USING TO_TIMESTAMP(last_activity);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE password_reset_request
		DROP CONSTRAINT password_reset_request_token_expiry_chk_unsigned;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE password_reset_request
		ALTER COLUMN token_expiry TYPE TIMESTAMPTZ
		USING TO_TIMESTAMP(token_expiry);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE docker_registry_repository_manifest
		DROP CONSTRAINT docker_registry_repository_manifest_chk_created_unsigned;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE docker_registry_repository_manifest
		ALTER COLUMN created TYPE TIMESTAMPTZ
		USING TO_TIMESTAMP(created);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE docker_registry_repository_tag
		DROP CONSTRAINT docker_registry_repository_tag_chk_last_updated_unsigned;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE docker_registry_repository_tag
		ALTER COLUMN last_updated TYPE TIMESTAMPTZ
		USING TO_TIMESTAMP(last_updated);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// Globally expire all user tokens, so that they're all generated again
	let key = "token-revoked:global";
	let schema = if config.redis.secure {
		"rediss"
	} else {
		"redis"
	};
	let username = config.redis.user.as_deref().unwrap_or("");
	let password = config
		.redis
		.password
		.as_deref()
		.map_or_else(|| "".to_string(), |pwd| format!(":{}@", pwd));
	let host = config.redis.host.as_str();
	let port = config.redis.port;
	let database = config.redis.database.unwrap_or(0);

	let mut redis = Client::open(format!(
		"{schema}://{username}{password}{host}:{port}/{database}"
	))?
	.get_multiplexed_tokio_connection()
	.await?;
	redis
		.set_ex(
			key,
			Utc::now().timestamp_millis(),
			Duration::days(30).num_seconds() as usize,
		)
		.await?;

	Ok(())
}

async fn create_user_transferring_domain_to_patr_table(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		CREATE TABLE user_transferring_domain_to_patr(
			domain_id UUID NOT NULL
				CONSTRAINT user_transfer_domain_pk PRIMARY KEY,
			nameserver_type DOMAIN_NAMESERVER_TYPE NOT NULL
				CONSTRAINT user_transfer_domain_chk_nameserver_type CHECK(
					nameserver_type = 'external'
				),
			zone_identifier TEXT NOT NULL,
			is_verified BOOLEAN NOT NULL,
			CONSTRAINT user_transfer_domain_fk_domain_id_nameserver_type
				FOREIGN KEY(domain_id)REFERENCES
					user_controlled_domain(domain_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_controlled_domain
		ADD COLUMN transferring_domain UUID;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_controlled_domain
		ADD CONSTRAINT user_controlled_domain_chk_transferring_domain_is_domain_id
		CHECK(transferring_domain = domain_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
