use crate::{
	migrate_query as query,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	add_processed_to_static_site_uploads(connection, config).await?;

	// ci-v3 migrations
	add_permission_for_write_ci_file_to_repo(connection, config).await?;
	add_ci_runner(connection, config).await?;

	// common post-migrations
	reset_permission_order(&mut *connection, config).await?;

	Ok(())
}

pub async fn add_processed_to_static_site_uploads(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE static_site_upload_history
		ADD COLUMN processed TIMESTAMPTZ;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			static_site_upload_history
		SET
			processed = created;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn add_permission_for_write_ci_file_to_repo(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	// add permissions for CI
	let permission = "workspace::ci::git_provider::repo::write";

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

	Ok(())
}

async fn reset_permission_order(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	for permission in [
		"workspace::domain::list",
		"workspace::domain::add",
		"workspace::domain::viewDetails",
		"workspace::domain::verify",
		"workspace::domain::delete",
		"workspace::domain::dnsRecord::list",
		"workspace::domain::dnsRecord::add",
		"workspace::domain::dnsRecord::edit",
		"workspace::domain::dnsRecord::delete",
		"workspace::infrastructure::deployment::list",
		"workspace::infrastructure::deployment::create",
		"workspace::infrastructure::deployment::info",
		"workspace::infrastructure::deployment::delete",
		"workspace::infrastructure::deployment::edit",
		"workspace::infrastructure::upgradePath::list",
		"workspace::infrastructure::upgradePath::create",
		"workspace::infrastructure::upgradePath::info",
		"workspace::infrastructure::upgradePath::delete",
		"workspace::infrastructure::upgradePath::edit",
		"workspace::infrastructure::managedUrl::list",
		"workspace::infrastructure::managedUrl::create",
		"workspace::infrastructure::managedUrl::edit",
		"workspace::infrastructure::managedUrl::delete",
		"workspace::infrastructure::managedDatabase::create",
		"workspace::infrastructure::managedDatabase::list",
		"workspace::infrastructure::managedDatabase::delete",
		"workspace::infrastructure::managedDatabase::info",
		"workspace::infrastructure::staticSite::list",
		"workspace::infrastructure::staticSite::create",
		"workspace::infrastructure::staticSite::info",
		"workspace::infrastructure::staticSite::delete",
		"workspace::infrastructure::staticSite::edit",
		"workspace::dockerRegistry::create",
		"workspace::dockerRegistry::list",
		"workspace::dockerRegistry::delete",
		"workspace::dockerRegistry::info",
		"workspace::dockerRegistry::push",
		"workspace::dockerRegistry::pull",
		"workspace::secret::list",
		"workspace::secret::create",
		"workspace::secret::edit",
		"workspace::secret::delete",
		"workspace::rbac::role::list",
		"workspace::rbac::role::create",
		"workspace::rbac::role::edit",
		"workspace::rbac::role::delete",
		"workspace::rbac::user::list",
		"workspace::rbac::user::add",
		"workspace::rbac::user::remove",
		"workspace::rbac::user::updateRoles",
		"workspace::region::list",
		"workspace::region::info",
		"workspace::region::add",
		"workspace::region::delete",
		"workspace::ci::git_provider::connect",
		"workspace::ci::git_provider::disconnect",
		"workspace::ci::git_provider::repo::activate",
		"workspace::ci::git_provider::repo::deactivate",
		"workspace::ci::git_provider::repo::list",
		"workspace::ci::git_provider::repo::info",
		"workspace::ci::git_provider::repo::write",
		"workspace::ci::git_provider::repo::build::list",
		"workspace::ci::git_provider::repo::build::cancel",
		"workspace::ci::git_provider::repo::build::info",
		"workspace::ci::git_provider::repo::build::start",
		"workspace::ci::git_provider::repo::build::restart",
		"workspace::billing::info",
		"workspace::billing::make_payment",
		"workspace::billing::payment_method::add",
		"workspace::billing::payment_method::delete",
		"workspace::billing::payment_method::list",
		"workspace::billing::payment_method::edit",
		"workspace::billing::billing_address::add",
		"workspace::billing::billing_address::delete",
		"workspace::billing::billing_address::info",
		"workspace::billing::billing_address::edit",
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

// migration note:
// 	before migration make sure no ci jobs are present in mq
async fn add_ci_runner(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE ci_repos
		DROP CONSTRAINT ci_repos_chk_status_machine_type_id_webhook_secret;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE ci_repos
		DROP COLUMN build_machine_type_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DROP TABLE ci_build_machine_type;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE ci_build_machine_type (
			id 		UUID 		NOT NULL,
			cpu 	INTEGER 	NOT NULL, /* Multiples of ¼th vCPU */
			ram 	INTEGER 	NOT NULL, /* Multiples of ¼th GB RAM */
			volume 	INTEGER 	NOT NULL, /* Multiples of ¼th GB storage */

			CONSTRAINT ci_machint_type_pk PRIMARY KEY (id),

			CONSTRAINT ci_build_machine_type_chk_cpu_positive
				CHECK (cpu > 0),
			CONSTRAINT ci_build_machine_type_chk_ram_positive
				CHECK (ram > 0),
			CONSTRAINT ci_build_machine_type_chk_volume_positive
				CHECK (volume > 0)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE ci_runner (
			id              		UUID			NOT NULL,
			name            		CITEXT			NOT NULL,
			workspace_id    		UUID    		NOT NULL,
			region_id       		UUID    		NOT NULL,
			build_machine_type_id	UUID			NOT NULL,
			deleted					TIMESTAMPTZ,

			CONSTRAINT ci_runner_pk PRIMARY KEY (id),

			CONSTRAINT ci_runner_fk_workspace_id
				FOREIGN KEY (workspace_id) REFERENCES workspace(id),
			CONSTRAINT ci_runner_fk_region_id
				FOREIGN KEY (region_id) REFERENCES deployment_region(id),
			CONSTRAINT ci_runner_fk_build_machine_type_id
				FOREIGN KEY (build_machine_type_id) REFERENCES ci_build_machine_type(id),
			CONSTRAINT ci_runner_fk_id_workspace_id
				FOREIGN KEY(id, workspace_id) REFERENCES resource(id, owner_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE UNIQUE INDEX ci_runner_uq_workspace_id_name
		ON ci_runner (workspace_id, name)
			WHERE deleted IS NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE ci_repos
		ADD COLUMN runner_id UUID
			CONSTRAINT ci_repos_fk_runner_id REFERENCES ci_runner(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE ci_repos
		SET status = 'inactive'
		WHERE status = 'active';
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE ci_repos
		ADD CONSTRAINT ci_repos_chk_status_runner_id_webhook_secret
			CHECK(
				(
					status = 'active'
					AND runner_id IS NOT NULL
					AND webhook_secret IS NOT NULL
				) OR
				status != 'active'
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TYPE ci_build_status
		ADD VALUE 'waiting_to_start' BEFORE 'running';
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE ci_builds
		ALTER COLUMN created SET DEFAULT NOW();
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE ci_builds
		ADD COLUMN started TIMESTAMPTZ;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE ci_builds
		ADD COLUMN runner_id UUID NOT NULL
			CONSTRAINT ci_builds_fk_runner_id REFERENCES ci_runner(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// add ci_runner as a resource type
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
				($1, 'ciRunner', '');
			"#,
		&resource_type_id
	)
	.execute(&mut *connection)
	.await?;

	// add permissions for CI runner
	for &permission in [
		"workspace::ci::runner::list",
		"workspace::ci::runner::create",
		"workspace::ci::runner::info",
		"workspace::ci::runner::update",
		"workspace::ci::runner::delete",
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

	// machine types for ci builds
	const CI_BUILD_MACHINE_TYPES: [(i32, i32, i32); 5] = [
		(4, 2, 4),    // 1 vCPU, 0.5 GB RAM, 1GB storage
		(4, 4, 4),    // 1 vCPU,   1 GB RAM, 1GB storage
		(4, 8, 8),    // 1 vCPU,   2 GB RAM, 2GB storage
		(8, 16, 16),  // 2 vCPU,   4 GB RAM, 4GB storage
		(16, 32, 32), // 4 vCPU,   8 GB RAM, 8GB storage
	];

	for (cpu, ram, volume) in CI_BUILD_MACHINE_TYPES {
		let machine_type_id =
			db::generate_new_resource_id(&mut *connection).await?;

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

	Ok(())
}