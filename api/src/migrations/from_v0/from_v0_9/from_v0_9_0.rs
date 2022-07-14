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
	add_migrations_for_ci(connection, config).await?;

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

async fn add_migrations_for_ci(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		CREATE TABLE ci_build_machine_type (
			id       UUID CONSTRAINT ci_machint_type_pk PRIMARY KEY,
			cpu      INTEGER NOT NULL CONSTRAINT ci_build_machine_type_chk_cpu_positive CHECK (cpu > 0),   		/* Multiples of 1 vCPU */
			ram      INTEGER NOT NULL CONSTRAINT ci_build_machine_type_chk_ram_positive CHECK (ram > 0),   		/* Multiples of 0.25 GB RAM */
			volume   INTEGER NOT NULL CONSTRAINT ci_build_machine_type_chk_volume_positive CHECK (volume > 0) 	/* Multiples of 1 GB storage space */
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	const CI_BUILD_MACHINE_TYPES: [(i32, i32, i32); 5] = [
		(1, 2, 1),  // 1 vCPU, 0.5 GB RAM, 1GB storage
		(1, 4, 1),  // 1 vCPU,   1 GB RAM, 1GB storage
		(1, 6, 2),  // 1 vCPU,   2 GB RAM, 2GB storage
		(2, 8, 4),  // 2 vCPU,   4 GB RAM, 4GB storage
		(4, 32, 8), // 4 vCPU,   8 GB RAM, 8GB storage
	];

	for (cpu, ram, volume) in CI_BUILD_MACHINE_TYPES {
		let machine_type_id = loop {
			let uuid = Uuid::new_v4();

			let exists = query!(
				r#"
					SELECT
						*
					FROM
						resource
					WHERE
						id = $1;
					"#,
				&uuid
			)
			.fetch_optional(&mut *connection)
			.await?
			.is_some();

			if !exists {
				break uuid;
			}
		};

		query!(
			r#"
			INSERT INTO 
				ci_build_machine_type(
					id,
					cpu,
					ram,
					volume
				)
			VALUES
				($1, $2, $3, $4);
			"#,
			machine_type_id,
			cpu,
			ram,
			volume
		)
		.execute(&mut *connection)
		.await?;
	}

	query!(
		r#"
		CREATE TABLE ci_repos (
			id 				UUID CONSTRAINT ci_repos_pk PRIMARY KEY,
			workspace_id 	UUID NOT NULL,
			repo_owner 		TEXT NOT NULL,
			repo_name 		TEXT NOT NULL,
			git_url 		TEXT NOT NULL,
			webhook_secret 	TEXT NOT NULL CONSTRAINT ci_repos_uq_secret UNIQUE,
			active 			BOOLEAN NOT NULL,

			CONSTRAINT ci_repos_fk_workspace_id
				FOREIGN KEY (workspace_id) REFERENCES workspace(id),
			CONSTRAINT ci_repos_uq_workspace_id_git_url
				UNIQUE (workspace_id, git_url),
			CONSTRAINT ci_repos_uq_workspace_id_repo_owner_repo_name
				UNIQUE (workspace_id, repo_owner, repo_name)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE CI_BUILD_STATUS AS ENUM (
			'running',
			'succeeded',
			'errored'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE ci_builds (
			repo_id		UUID NOT NULL CONSTRAINT ci_builds_fk_repo_id REFERENCES ci_repos(id),
			build_num 	BIGINT NOT NULL CONSTRAINT ci_builds_chk_build_num_unsigned CHECK (build_num > 0),
			git_ref 	TEXT NOT NULL,
			git_commit 	TEXT NOT NULL,
			status 		CI_BUILD_STATUS NOT NULL,
			created 	TIMESTAMPTZ NOT NULL,
			finished 	TIMESTAMPTZ,

			CONSTRAINT ci_builds_pk_repo_id_build_num
				PRIMARY KEY (repo_id, build_num)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE CI_BUILD_STEP_STATUS AS ENUM (
			'waiting_to_start',
			'running',
			'succeeded',
			'errored',
			'skipped_dep_error'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE ci_steps (
			repo_id 	UUID NOT NULL,
			build_num 	BIGINT NOT NULL,
			step_id 	INTEGER NOT NULL CONSTRAINT ci_steps_chk_step_id_unsigned CHECK (build_num >= 0),
			step_name 	TEXT NOT NULL,
			base_image 	TEXT NOT NULL,
			commands 	TEXT[] NOT NULL,
			status 		CI_BUILD_STEP_STATUS NOT NULL,
			started 	TIMESTAMPTZ,
			finished 	TIMESTAMPTZ,

			CONSTRAINT ci_steps_fk_repo_id_build_num
				FOREIGN KEY (repo_id, build_num) REFERENCES ci_builds(repo_id, build_num),
			CONSTRAINT ci_steps_pk_repo_id_build_num_step_id
				PRIMARY KEY (repo_id, build_num, step_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}