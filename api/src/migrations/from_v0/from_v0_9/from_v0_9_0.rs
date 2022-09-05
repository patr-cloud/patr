use api_models::utils::Uuid;
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
	migrate_from_bigint_to_timestamptz(&mut *connection, config).await?;
	add_migrations_for_ci(&mut *connection, config).await?;
	clean_up_drone_ci(&mut *connection, config).await?;
	reset_permission_order(&mut *connection, config).await?;

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
		CREATE TYPE CI_GIT_PROVIDER_TYPE AS ENUM(
			'github'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE ci_git_provider (
			id 					UUID CONSTRAINT ci_git_provider_pk PRIMARY KEY,
			workspace_id 		UUID NOT NULL CONSTRAINT ci_git_provider_fk_workspace_id REFERENCES workspace(id),
			domain_name 		TEXT NOT NULL,
			git_provider_type 	CI_GIT_PROVIDER_TYPE NOT NULL,
			login_name 			TEXT,
			password 			TEXT,
			is_deleted			BOOL NOT NULL DEFAULT FALSE,

			CONSTRAINT ci_git_provider_ch_login_name_password_is_deleted
				CHECK (
					(is_deleted = TRUE AND password IS NULL)
					OR (
						is_deleted = FALSE
						AND (
							(password IS NOT NULL AND login_name IS NOT NULL)
							OR (password IS NULL)
					))
				)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE UNIQUE INDEX
			ci_git_provider_uq_workspace_id_domain_name
		ON
			ci_git_provider(workspace_id, domain_name)
		WHERE
			is_deleted = FALSE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE CI_REPO_STATUS AS ENUM (
			'active',
			'inactive',
			'deleted'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE ci_repos (
			id 						UUID CONSTRAINT ci_repos_pk PRIMARY KEY,
			repo_owner 				TEXT NOT NULL,
			repo_name 				TEXT NOT NULL,
			clone_url 				TEXT NOT NULL,
			webhook_secret 			TEXT CONSTRAINT ci_repos_uq_secret UNIQUE,
			build_machine_type_id 	UUID CONSTRAINT ci_repos_fk_build_machine_type_id REFERENCES ci_build_machine_type(id),
			status 					CI_REPO_STATUS NOT NULL,
			git_provider_id 		UUID NOT NULL CONSTRAINT ci_repos_fk_git_provider_id REFERENCES ci_git_provider(id),
			git_provider_repo_uid 	TEXT NOT NULL,

			CONSTRAINT ci_repos_uq_git_provider_id_repo_uid
				UNIQUE (git_provider_id, git_provider_repo_uid),
			CONSTRAINT ci_repos_chk_status_machine_type_id_webhook_secret
				CHECK (
					(
						status = 'active'
						AND build_machine_type_id IS NOT NULL
						AND webhook_secret IS NOT NULL
					)
					OR status != 'active'
				)
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
			'cancelled',
			'errored'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE ci_builds (
			repo_id 	UUID NOT NULL CONSTRAINT ci_builds_fk_repo_id REFERENCES ci_repos(id),
			build_num 	BIGINT NOT NULL CONSTRAINT ci_builds_chk_build_num_unsigned CHECK (build_num > 0),
			git_ref 	TEXT NOT NULL,
			git_commit 	TEXT NOT NULL,
			status 		CI_BUILD_STATUS NOT NULL,
			created 	TIMESTAMPTZ NOT NULL,
			finished 	TIMESTAMPTZ,
			message		TEXT,

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
			'cancelled',
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
			commands 	TEXT NOT NULL,
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

	// add ci_repo as a resource type
	let resource_type_id = loop {
		let resource_type_id = Uuid::new_v4();

		let exists = query!(
			r#"
				SELECT
					*
				FROM
					resource_type
				WHERE
					id = $1;
				"#,
			&resource_type_id
		)
		.fetch_optional(&mut *connection)
		.await?
		.is_some();

		if !exists {
			break resource_type_id;
		}
	};

	query!(
		r#"
			INSERT INTO
				resource_type(
					id,
					name,
					description
				)
			VALUES
				($1, 'ciRepo', '');
			"#,
		&resource_type_id
	)
	.execute(&mut *connection)
	.await?;

	// add permissions for CI
	for &permission in [
		"workspace::ci::git_provider::connect",
		"workspace::ci::git_provider::disconnect",
		"workspace::ci::git_provider::repo::activate",
		"workspace::ci::git_provider::repo::deactivate",
		"workspace::ci::git_provider::repo::list",
		"workspace::ci::git_provider::repo::build::view",
		"workspace::ci::git_provider::repo::build::restart",
	]
	.iter()
	{
		let uuid = loop {
			let uuid = Uuid::new_v4();

			let exists = query!(
				r#"
				SELECT
					*
				FROM
					permission
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
				permission
			VALUES
				($1, $2, '');
			"#,
			&uuid,
			permission
		)
		.fetch_optional(&mut *connection)
		.await?;
	}

	Ok(())
}

async fn clean_up_drone_ci(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	// drop drone token columns in workspace table
	query!(
		r#"
		ALTER TABLE workspace
			DROP COLUMN drone_username,
			DROP COLUMN drone_token;
		"#
	)
	.execute(&mut *connection)
	.await?;

	// remove permissions
	for &permission in [
		"workspace::ci::github::connect",
		"workspace::ci::github::activate",
		"workspace::ci::github::deactivate",
		"workspace::ci::github::viewBuilds",
		"workspace::ci::github::restartBuilds",
		"workspace::ci::github::disconnect",
	]
	.iter()
	{
		query!(
			r#"
			DELETE FROM
				permission
			WHERE
				name = $1;
			"#,
			permission
		)
		.execute(&mut *connection)
		.await?;
	}

	Ok(())
}

async fn reset_permission_order(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	for permission in [
		// domain
		"workspace::domain::list",
		"workspace::domain::add",
		"workspace::domain::viewDetails",
		"workspace::domain::verify",
		"workspace::domain::delete",
		// dns
		"workspace::domain::dnsRecord::list",
		"workspace::domain::dnsRecord::add",
		"workspace::domain::dnsRecord::edit",
		"workspace::domain::dnsRecord::delete",
		// deployment
		"workspace::infrastructure::deployment::list",
		"workspace::infrastructure::deployment::create",
		"workspace::infrastructure::deployment::info",
		"workspace::infrastructure::deployment::delete",
		"workspace::infrastructure::deployment::edit",
		// upgrade path
		"workspace::infrastructure::upgradePath::list",
		"workspace::infrastructure::upgradePath::create",
		"workspace::infrastructure::upgradePath::info",
		"workspace::infrastructure::upgradePath::delete",
		"workspace::infrastructure::upgradePath::edit",
		// managed url
		"workspace::infrastructure::managedUrl::list",
		"workspace::infrastructure::managedUrl::create",
		"workspace::infrastructure::managedUrl::edit",
		"workspace::infrastructure::managedUrl::delete",
		// managed database
		"workspace::infrastructure::managedDatabase::create",
		"workspace::infrastructure::managedDatabase::list",
		"workspace::infrastructure::managedDatabase::delete",
		"workspace::infrastructure::managedDatabase::info",
		// static site
		"workspace::infrastructure::staticSite::list",
		"workspace::infrastructure::staticSite::create",
		"workspace::infrastructure::staticSite::info",
		"workspace::infrastructure::staticSite::delete",
		"workspace::infrastructure::staticSite::edit",
		// docker registry
		"workspace::dockerRegistry::create",
		"workspace::dockerRegistry::list",
		"workspace::dockerRegistry::delete",
		"workspace::dockerRegistry::info",
		"workspace::dockerRegistry::push",
		"workspace::dockerRegistry::pull",
		// secret
		"workspace::secret::list",
		"workspace::secret::create",
		"workspace::secret::edit",
		"workspace::secret::delete",
		// role
		"workspace::rbac::role::list",
		"workspace::rbac::role::create",
		"workspace::rbac::role::edit",
		"workspace::rbac::role::delete",
		// user
		"workspace::rbac::user::list",
		"workspace::rbac::user::add",
		"workspace::rbac::user::remove",
		"workspace::rbac::user::updateRoles",
		// ci (old)
		"workspace::ci::github::connect",
		"workspace::ci::github::activate",
		"workspace::ci::github::deactivate",
		"workspace::ci::github::viewBuilds",
		"workspace::ci::github::restartBuilds",
		"workspace::ci::github::disconnect",
		// ci (new)
		"workspace::ci::git_provider::connect",
		"workspace::ci::git_provider::disconnect",
		"workspace::ci::git_provider::repo::activate",
		"workspace::ci::git_provider::repo::deactivate",
		"workspace::ci::git_provider::repo::list",
		"workspace::ci::git_provider::repo::build::view",
		"workspace::ci::git_provider::repo::build::restart",
		// workspace
		"workspace::edit",
		"workspace::delete",
	] {
		query!(
			r#"
			UPDATE
				permission
			SET
				name = CONCAT('test::', name)
			WHERE
				name = $1;
			"#,
			permission,
		)
		.execute(&mut *connection)
		.await?;

		query!(
			r#"
			UPDATE
				permission
			SET
				name = $1
			WHERE
				name = CONCAT('test::', $1);
			"#,
			&permission,
		)
		.execute(&mut *connection)
		.await?;
	}

	Ok(())
}
