use api_models::utils::Uuid;
use sqlx::Row;

use crate::{
	db,
	migrate_query as query,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	add_tables_for_k8s_database(connection, config).await?;
	add_machine_type_for_k8s_database(connection, config).await?;
	update_permissions(connection, config).await?;
	re_add_constraints(connection, config).await?;
	remove_list_permissions(connection, config).await?;
	add_rbac_blocklist_tables(connection, config).await?;
	reset_permission_order(connection, config).await?;

	Ok(())
}

async fn add_tables_for_k8s_database(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		DROP TYPE MANAGED_DATABASE_ENGINE CASCADE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DROP TABLE managed_database;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TYPE MANAGED_DATABASE_PLAN
		RENAME TO LEGACY_MANAGED_DATABASE_PLAN;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DROP TYPE MANAGED_DATABASE_STATUS;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE MANAGED_DATABASE_ENGINE AS ENUM(
			'postgres',
			'mysql',
			'mongo',
			'redis'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE managed_database_plan(
			id 		UUID CONSTRAINT managed_database_plan_pk PRIMARY KEY,
			cpu 	INTEGER NOT NULL, /* Multiples of 1 vCPU */
			ram 	INTEGER NOT NULL, /* Multiples of 1 GB RAM */
			volume 	INTEGER NOT NULL, /* Multiples of 1 GB storage space */

			CONSTRAINT managed_database_plan_chk_cpu_positive CHECK(cpu > 0),
			CONSTRAINT managed_database_plan_chk_ram_positive CHECK(ram > 0),
			CONSTRAINT managed_database_plan_chk_volume_positive
				CHECK(volume > 0)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE managed_database_payment_history
			ADD COLUMN db_plan_id UUID,
			ADD COLUMN legacy_db_plan LEGACY_MANAGED_DATABASE_PLAN,
			ADD COLUMN new_start_time TIMESTAMPTZ,
			ADD COLUMN new_deletion_time TIMESTAMPTZ,
			ADD CONSTRAINT managed_database_payment_history_fk_db_plan_id
				FOREIGN KEY(db_plan_id) REFERENCES managed_database_plan(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			managed_database_payment_history
		SET
			legacy_db_plan = db_plan,
			new_start_time = managed_database_payment_history.start_time,
			new_deletion_time = managed_database_payment_history.deletion_time;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE managed_database_payment_history
			DROP COLUMN db_plan,
			DROP COLUMN start_time,
			DROP COLUMN deletion_time,
			ALTER COLUMN new_start_time SET NOT NULL,
			ADD CONSTRAINT managed_database_payment_history_chk_legacy_db_plan
				CHECK(
					(
						db_plan_id IS NULL AND
						legacy_db_plan IS NOT NULL
					) OR (
						db_plan_id IS NOT NULL AND
						legacy_db_plan IS NULL
					)
				);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE managed_database_payment_history
		RENAME COLUMN new_start_time TO start_time;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE managed_database_payment_history
		RENAME COLUMN new_deletion_time TO deletion_time;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE MANAGED_DATABASE_STATUS AS ENUM(
			'creating',
			'running',
			'errored',
			'deleted'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE managed_database(
			id UUID CONSTRAINT managed_database_pk PRIMARY KEY,
			name CITEXT NOT NULL,
			workspace_id UUID NOT NULL,
			region UUID NOT NULL,
			engine MANAGED_DATABASE_ENGINE NOT NULL,
			database_plan_id UUID NOT NULL,
			status MANAGED_DATABASE_STATUS NOT NULL,
			username TEXT NOT NULL,
			deleted TIMESTAMPTZ,

			CONSTRAINT managed_database_chk_name_is_trimmed CHECK(
				name = TRIM(name)
			),
			CONSTRAINT managed_database_fk_workspace_id
				FOREIGN KEY(workspace_id) REFERENCES workspace(id),
			CONSTRAINT managed_database_fk_region
				FOREIGN KEY(region) REFERENCES region(id),
			CONSTRAINT managed_database_fk_managed_database_plan_id
				FOREIGN KEY(database_plan_id)
					REFERENCES managed_database_plan(id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE UNIQUE INDEX
			managed_database_uq_workspace_id_name
		ON
			managed_database(workspace_id, name)
		WHERE
			deleted IS NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE managed_database
		ADD CONSTRAINT managed_database_fk_id_workspace_id
		FOREIGN KEY(id, workspace_id) REFERENCES resource(id, owner_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn add_machine_type_for_k8s_database(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	const MANAGED_DATABASE_MACHINE_TYPE: [(i32, i32, i32); 4] = [
		(1, 2, 25),  // 1 vCPU, 2 GB RAM, 25GB storage
		(2, 4, 50),  // 2 vCPU, 4 GB RAM, 50GB storage
		(2, 4, 100), // 2 vCPU, 4 GB RAM, 100GB storage
		(4, 8, 200), // 4 vCPU, 8 GB RAM, 200GB storage
	];

	for (cpu, ram, volume) in MANAGED_DATABASE_MACHINE_TYPE {
		let machine_type_id =
			db::generate_new_resource_id(&mut *connection).await?;
		query!(
			r#"
			INSERT INTO
				managed_database_plan(
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

async fn add_rbac_blocklist_tables(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	// Add RBAC tables for role
	query!(
		r#"
		CREATE TYPE PERMISSION_TYPE AS ENUM(
			'include',
			'exclude'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE role_resource_permissions_type(
			role_id 		UUID 			NOT NULL,
			permission_id 	UUID 			NOT NULL,
			permission_type PERMISSION_TYPE NOT NULL,
			CONSTRAINT role_resource_permissions_type_pk PRIMARY KEY(
				role_id,
				permission_id
			),
			CONSTRAINT role_resource_permissions_type_uq UNIQUE(
				role_id,
				permission_id,
				permission_type
			),
			CONSTRAINT role_resource_permissions_type_fk_role_id
				FOREIGN KEY(role_id) REFERENCES role(id),
			CONSTRAINT role_resource_permissions_type_fk_permission_id
				FOREIGN KEY(permission_id) REFERENCES permission(id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE role_resource_permissions_include(
			role_id 		UUID 			NOT NULL,
			permission_id 	UUID 			NOT NULL,
			resource_id		UUID			NOT NULL,
			permission_type PERMISSION_TYPE NOT NULL
				GENERATED ALWAYS AS('include') STORED,

			CONSTRAINT role_resource_permissions_include_pk PRIMARY KEY(
				role_id,
				permission_id,
				resource_id
			),
			CONSTRAINT role_resource_permissions_include_fk_parent FOREIGN KEY(
				role_id,
				permission_id,
				permission_type
			) REFERENCES role_resource_permissions_type(
				role_id,
				permission_id,
				permission_type
			),
			CONSTRAINT role_resource_permissions_include_fk_resource
				FOREIGN KEY(resource_id) REFERENCES resource(id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE role_resource_permissions_exclude(
			role_id 		UUID 			NOT NULL,
			permission_id 	UUID 			NOT NULL,
			resource_id		UUID			NOT NULL,
			permission_type PERMISSION_TYPE NOT NULL
				GENERATED ALWAYS AS ('exclude') STORED,

			CONSTRAINT role_resource_permissions_exclude_pk PRIMARY KEY(
				role_id,
				permission_id,
				resource_id
			),
			CONSTRAINT role_resource_permissions_exclude_fk_parent
				FOREIGN KEY(role_id, permission_id, permission_type)
					REFERENCES role_resource_permissions_type(
						role_id,
						permission_id,
						permission_type
					),
			CONSTRAINT role_resource_permissions_exclude_fk_resource
				FOREIGN KEY(resource_id) REFERENCES resource(id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		INSERT INTO
			role_resource_permissions_type(
				role_id,
				permission_id,
				permission_type
			)
		SELECT
			role_id,
			permission_id,
			'exclude'
		FROM
			role_permissions_resource_type
		ON CONFLICT DO NOTHING;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		INSERT INTO
			role_resource_permissions_type(
				role_id,
				permission_id,
				permission_type
			)
		SELECT
			role_id,
			permission_id,
			'include'
		FROM
			role_permissions_resource
		ON CONFLICT DO NOTHING;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		INSERT INTO
			role_resource_permissions_include(
				role_id,
				permission_id,
				resource_id
			)
		SELECT
			role_id,
			permission_id,
			resource_id
		FROM
			role_permissions_resource
		WHERE EXISTS (
			SELECT 1
			FROM role_resource_permissions_type
			WHERE 
				role_id = role_permissions_resource.role_id
				AND permission_id = role_permissions_resource.permission_id
				AND permission_type = 'include'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DROP TABLE role_permissions_resource;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DROP TABLE role_permissions_resource_type;
		"#
	)
	.execute(&mut *connection)
	.await?;

	// Add RBAC tables for user API token
	query!(
		r#"
		CREATE TYPE TOKEN_PERMISSION_TYPE AS ENUM(
			'super_admin',
			'member'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE user_api_token_workspace_permission_type(
			token_id 				UUID 					NOT NULL,
			workspace_id			UUID 					NOT NULL,
			token_permission_type 	TOKEN_PERMISSION_TYPE 	NOT NULL,
			CONSTRAINT user_api_token_workspace_permission_type_pk PRIMARY KEY(
				token_id,
				workspace_id
			),
			CONSTRAINT user_api_token_workspace_permission_type_uq UNIQUE(
				token_id,
				workspace_id,
				token_permission_type
			),
			CONSTRAINT user_api_token_workspace_permission_type_fk_token
				FOREIGN KEY(token_id) REFERENCES user_api_token(token_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_api_token_workspace_super_admin
		ADD COLUMN token_permission_type TOKEN_PERMISSION_TYPE NOT NULL
		GENERATED ALWAYS AS ('super_admin') STORED;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_api_token_workspace_super_admin
		ADD CONSTRAINT user_api_token_workspace_super_admin_pk
		PRIMARY KEY(token_id, user_id, workspace_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		INSERT INTO
			user_api_token_workspace_permission_type
		SELECT
			token_id,
			workspace_id,
			'super_admin'
		FROM
			user_api_token_workspace_super_admin;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		INSERT INTO
			user_api_token_workspace_permission_type
		SELECT
			user_api_token.token_id,
			COALESCE(
				user_api_token_resource_type_permission.workspace_id,
				user_api_token_resource_permission.workspace_id
			),
			'member'
		FROM
			user_api_token
		LEFT JOIN
			user_api_token_resource_permission
		ON
			user_api_token_resource_permission.token_id = user_api_token.token_id
		LEFT JOIN
			user_api_token_resource_type_permission
		ON
			user_api_token_resource_type_permission.token_id = user_api_token.token_id
		WHERE
			user_api_token.token_id NOT IN (
				SELECT
					token_id
				FROM
					user_api_token_workspace_super_admin
			)
			AND COALESCE(
				user_api_token_resource_type_permission.workspace_id,
				user_api_token_resource_permission.workspace_id
			) IS NOT NULL
		ON CONFLICT DO NOTHING;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_api_token_workspace_super_admin
		ADD CONSTRAINT user_api_token_workspace_super_admin_fk_type
		FOREIGN KEY(token_id, workspace_id, token_permission_type)
		REFERENCES user_api_token_workspace_permission_type(
			token_id,
			workspace_id,
			token_permission_type
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE user_api_token_resource_permissions_type(
			token_id 					UUID 			NOT NULL,
			workspace_id 				UUID 			NOT NULL,
			permission_id 				UUID 			NOT NULL,
			resource_permission_type	PERMISSION_TYPE NOT NULL,
			token_permission_type TOKEN_PERMISSION_TYPE NOT NULL
				GENERATED ALWAYS AS ('member') STORED,
			CONSTRAINT user_api_token_resource_permissions_type_pk PRIMARY KEY(
				token_id,
				workspace_id,
				permission_id
			),
			CONSTRAINT user_api_token_resource_permissions_type_uq UNIQUE(
				token_id,
				workspace_id,
				permission_id,
				resource_permission_type
			),
			CONSTRAINT user_api_token_resource_permisssions_type_fk_type
				FOREIGN KEY(token_id, workspace_id, token_permission_type)
					REFERENCES user_api_token_workspace_permission_type(
						token_id,
						workspace_id,
						token_permission_type
					),
			CONSTRAINT
				user_api_token_resource_permisssions_type_fk_permission_id
					FOREIGN KEY(permission_id) REFERENCES permission(id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE user_api_token_resource_permissions_include(
			token_id 		UUID 			NOT NULL,
			workspace_id 	UUID 			NOT NULL,
			permission_id 	UUID 			NOT NULL,
			resource_id		UUID			NOT NULL,
			permission_type PERMISSION_TYPE NOT NULL
				GENERATED ALWAYS AS ('include') STORED,
			CONSTRAINT user_api_token_resource_permissions_include_pk
				PRIMARY KEY(token_id, workspace_id, permission_id, resource_id),
			CONSTRAINT user_api_token_resource_permissions_include_fk_parent
				FOREIGN KEY(
					token_id,
					workspace_id,
					permission_id,
					permission_type
				) REFERENCES user_api_token_resource_permissions_type(
					token_id,
					workspace_id,
					permission_id,
					resource_permission_type
				),
			CONSTRAINT user_api_token_resource_permissions_include_fk_resource
				FOREIGN KEY(resource_id, workspace_id) REFERENCES resource(
					id,
					owner_id
				)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE user_api_token_resource_permissions_exclude(
			token_id 		UUID 			NOT NULL,
			workspace_id 	UUID 			NOT NULL,
			permission_id 	UUID 			NOT NULL,
			resource_id		UUID			NOT NULL,
			permission_type PERMISSION_TYPE NOT NULL
				GENERATED ALWAYS AS ('exclude') STORED,
			CONSTRAINT user_api_token_resource_permissions_exclude_pk
				PRIMARY KEY(token_id, workspace_id, permission_id, resource_id),
			CONSTRAINT user_api_token_resource_permissions_exclude_fk_parent
				FOREIGN KEY(
					token_id,
					workspace_id,
					permission_id,
					permission_type
				) REFERENCES user_api_token_resource_permissions_type(
					token_id,
					workspace_id,
					permission_id,
					resource_permission_type
				),
			CONSTRAINT user_api_token_resource_permissions_exclude_fk_resource
				FOREIGN KEY(resource_id, workspace_id) REFERENCES resource(
					id,
					owner_id
				)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		INSERT INTO
			user_api_token_resource_permissions_type(
				token_id,
				workspace_id,
				permission_id,
				resource_permission_type
			)
		SELECT
			token_id,
			workspace_id,
			permission_id,
			'exclude'
		FROM
			user_api_token_resource_type_permission
		WHERE EXISTS (
			SELECT 1
			FROM user_api_token_workspace_permission_type
			WHERE 
				token_id = user_api_token_resource_type_permission.token_id
				AND workspace_id = user_api_token_resource_type_permission.workspace_id
				AND token_permission_type = 'member'
		)
		ON CONFLICT DO NOTHING;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		INSERT INTO
			user_api_token_resource_permissions_type(
				token_id,
				workspace_id,
				permission_id,
				resource_permission_type
			)
		SELECT
			token_id,
			workspace_id,
			permission_id,
			'include'
		FROM
			user_api_token_resource_permission
		WHERE EXISTS (
			SELECT 1
			FROM user_api_token_workspace_permission_type
			WHERE 
				token_id = user_api_token_resource_permission.token_id
				AND workspace_id = user_api_token_resource_permission.workspace_id
				AND token_permission_type = 'member'
		)	
		ON CONFLICT DO NOTHING;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		INSERT INTO
			user_api_token_resource_permissions_include(
				token_id,
				workspace_id,
				permission_id,
				resource_id
			)
		SELECT
			token_id,
			workspace_id,
			permission_id,
			resource_id
		FROM
			user_api_token_resource_permission
		WHERE EXISTS (
			SELECT 1
			FROM user_api_token_workspace_permission_type
			WHERE 
				token_id = user_api_token_resource_permission.token_id
				AND workspace_id = user_api_token_resource_permission.workspace_id
				AND token_permission_type = 'member'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DROP TABLE user_api_token_resource_permission;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DROP TABLE user_api_token_resource_type_permission;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn update_permissions(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	for permission in [
		"workspace::infrastructure::upgradePath::list",
		"workspace::infrastructure::upgradePath::create",
		"workspace::infrastructure::upgradePath::info",
		"workspace::infrastructure::upgradePath::delete",
		"workspace::infrastructure::upgradePath::edit",
	] {
		query!(
			r#"
			DELETE FROM
				permission
			WHERE
				name = $1
			"#,
			permission
		)
		.execute(&mut *connection)
		.await?;
	}

	query!(
		r#"
		UPDATE
			permission
		SET
			name = REPLACE(
				name,
				'workspace::dockerRegistry::',
				'workspace::containerRegistry::'
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	for (old_name, new_name) in [
		(
			"workspace::region::check_status",
			"workspace::region::checkStatus",
		),
		(
			"workspace::ci::recent_activity",
			"workspace::ci::recentActivity",
		),
		(
			"workspace::billing::make_payment",
			"workspace::billing::makePayment",
		),
	] {
		query!(
			r#"
			UPDATE
				permission
			SET
				name = $2
			WHERE
				name = $1
			"#,
			old_name,
			new_name
		)
		.execute(&mut *connection)
		.await?;
	}

	query!(
		r#"
		UPDATE
			permission
		SET
			name = REPLACE(
				name,
				'workspace::ci::git_provider::',
				'workspace::ci::gitProvider::'
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			permission
		SET
			name = REPLACE(
				name,
				'workspace::billing::payment_method::',
				'workspace::billing::paymentMethod::'
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			permission
		SET
			name = REPLACE(
				name,
				'workspace::billing::billing_address::',
				'workspace::billing::billingAddress::'
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			resource_type
		SET
			name = 'containerRegistryRepository'
		WHERE
			name = 'dockerRepository';
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			resource_type
		SET
			name = 'region'
		WHERE
			name = 'deploymentRegion';
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DELETE FROM
			resource_type
		WHERE
			name = 'deploymentUpgradePath';
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn remove_list_permissions(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	// rename existing permissions
	query!(
		r#"
		UPDATE
			permission
		SET
			name = 'workspace::domain::info'
		WHERE
			name = 'workspace::domain::viewDetails';		
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// insert missing permissions
	for &permission in [
		"workspace::infrastructure::managedUrl::info",
		"workspace::secret::info",
		"workspace::ci::gitProvider::repo::sync",
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

	// migrate from list permissions to info permissions
	for (from_permision, to_permission) in [
		("workspace::domain::list", "workspace::domain::info"),
		(
			"workspace::infrastructure::deployment::list",
			"workspace::infrastructure::deployment::info",
		),
		(
			"workspace::infrastructure::managedDatabase::list",
			"workspace::infrastructure::managedDatabase::info",
		),
		(
			"workspace::infrastructure::staticSite::list",
			"workspace::infrastructure::staticSite::info",
		),
		(
			"workspace::containerRegistry::list",
			"workspace::containerRegistry::info",
		),
		("workspace::region::list", "workspace::region::info"),
		("workspace::ci::runner::list", "workspace::ci::runner::info"),
		(
			"workspace::infrastructure::managedUrl::list",
			"workspace::infrastructure::managedUrl::info",
		),
		("workspace::secret::list", "workspace::secret::info"),
		(
			"workspace::ci::gitProvider::repo::list",
			"workspace::ci::gitProvider::repo::info",
		),
	]
	.iter()
	{
		let from_permision = query!(
			r#"
			SELECT
				id
			FROM
				permission
			WHERE
				name = $1;
			"#,
			from_permision
		)
		.fetch_one(&mut *connection)
		.await
		.map(|row| row.get::<Uuid, _>("id"))?;

		let to_permission = query!(
			r#"
			SELECT
				id
			FROM
				permission
			WHERE
				name = $1;
			"#,
			to_permission
		)
		.fetch_one(&mut *connection)
		.await
		.map(|row| row.get::<Uuid, _>("id"))?;

		query!(
			r#"
			DELETE FROM
				workspace_audit_log
			WHERE
				action = $1;
			"#,
			&from_permision
		)
		.execute(&mut *connection)
		.await?;

		query!(
			r#"
			DELETE FROM
				role_permissions_resource_type
			WHERE
				permission_id = $1;
			"#,
			&from_permision
		)
		.execute(&mut *connection)
		.await?;

		query!(
			r#"
			INSERT INTO
				role_permissions_resource_type(
					role_id,
					permission_id,
					resource_type_id
				)
			SELECT
				role_permissions_resource.role_id,
				$2,
				resource_type.id
			FROM
				role_permissions_resource
			JOIN
				resource
			ON
				resource.id = role_permissions_resource.resource_id
			JOIN
				resource_type
			ON
				(
					resource_type.name = 'workspace' AND
					resource_type.id = resource.resource_type_id
				)
			WHERE
				permission_id = $1
			ON CONFLICT DO NOTHING;
			"#,
			&from_permision,
			&to_permission,
		)
		.execute(&mut *connection)
		.await?;

		query!(
			r#"
			DELETE FROM
				role_permissions_resource
			WHERE
				permission_id = $1;
			"#,
			&from_permision
		)
		.execute(&mut *connection)
		.await?;

		query!(
			r#"
			DELETE FROM
				user_api_token_resource_type_permission
			WHERE
				permission_id = $1;
			"#,
			&from_permision
		)
		.execute(&mut *connection)
		.await?;

		query!(
			r#"
			INSERT INTO
				user_api_token_resource_type_permission(
					token_id,
					workspace_id,
					resource_type_id,
					permission_id
				)
			SELECT
				user_api_token_resource_permission.token_id,
				user_api_token_resource_permission.workspace_id,
				resource_type.id,
				$2
			FROM
				user_api_token_resource_permission
			JOIN
				resource
			ON
				resource.id = user_api_token_resource_permission.resource_id
			JOIN
				resource_type
			ON
				(
					resource_type.name = 'workspace' AND
					resource_type.id = resource.resource_type_id
				)
			WHERE
				permission_id = $1
			ON CONFLICT DO NOTHING;
			"#,
			&from_permision,
			&to_permission,
		)
		.execute(&mut *connection)
		.await?;

		query!(
			r#"
			DELETE FROM
				user_api_token_resource_permission
			WHERE
				permission_id = $1;
			"#,
			&from_permision
		)
		.execute(&mut *connection)
		.await?;
	}

	// delete list permissions
	query!(
		r#"
		DELETE FROM
			permission
		WHERE
			name IN (
				'workspace::domain::list',
				'workspace::infrastructure::deployment::list',
				'workspace::infrastructure::managedDatabase::list',
				'workspace::infrastructure::staticSite::list',
				'workspace::containerRegistry::list',
				'workspace::region::list',
				'workspace::ci::runner::list',
				'workspace::infrastructure::managedUrl::list',
				'workspace::secret::list',
				'workspace::ci::gitProvider::repo::list'
			);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn reset_permission_order(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	for permission in [
		"workspace::domain::add",
		"workspace::domain::info",
		"workspace::domain::verify",
		"workspace::domain::delete",
		"workspace::domain::dnsRecord::list",
		"workspace::domain::dnsRecord::add",
		"workspace::domain::dnsRecord::edit",
		"workspace::domain::dnsRecord::delete",
		"workspace::infrastructure::deployment::create",
		"workspace::infrastructure::deployment::info",
		"workspace::infrastructure::deployment::delete",
		"workspace::infrastructure::deployment::edit",
		"workspace::infrastructure::managedUrl::info",
		"workspace::infrastructure::managedUrl::create",
		"workspace::infrastructure::managedUrl::edit",
		"workspace::infrastructure::managedUrl::delete",
		"workspace::infrastructure::managedDatabase::create",
		"workspace::infrastructure::managedDatabase::delete",
		"workspace::infrastructure::managedDatabase::info",
		"workspace::infrastructure::staticSite::create",
		"workspace::infrastructure::staticSite::info",
		"workspace::infrastructure::staticSite::delete",
		"workspace::infrastructure::staticSite::edit",
		"workspace::containerRegistry::create",
		"workspace::containerRegistry::delete",
		"workspace::containerRegistry::info",
		"workspace::containerRegistry::push",
		"workspace::containerRegistry::pull",
		"workspace::secret::info",
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
		"workspace::region::info",
		"workspace::region::checkStatus",
		"workspace::region::add",
		"workspace::region::delete",
		"workspace::ci::recentActivity",
		"workspace::ci::gitProvider::list",
		"workspace::ci::gitProvider::connect",
		"workspace::ci::gitProvider::disconnect",
		"workspace::ci::gitProvider::repo::activate",
		"workspace::ci::gitProvider::repo::deactivate",
		"workspace::ci::gitProvider::repo::sync",
		"workspace::ci::gitProvider::repo::info",
		"workspace::ci::gitProvider::repo::write",
		"workspace::ci::gitProvider::repo::build::list",
		"workspace::ci::gitProvider::repo::build::cancel",
		"workspace::ci::gitProvider::repo::build::info",
		"workspace::ci::gitProvider::repo::build::start",
		"workspace::ci::gitProvider::repo::build::restart",
		"workspace::ci::runner::create",
		"workspace::ci::runner::info",
		"workspace::ci::runner::update",
		"workspace::ci::runner::delete",
		"workspace::billing::info",
		"workspace::billing::makePayment",
		"workspace::billing::paymentMethod::add",
		"workspace::billing::paymentMethod::delete",
		"workspace::billing::paymentMethod::list",
		"workspace::billing::paymentMethod::edit",
		"workspace::billing::billingAddress::add",
		"workspace::billing::billingAddress::delete",
		"workspace::billing::billingAddress::info",
		"workspace::billing::billingAddress::edit",
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
			permission,
		)
		.execute(&mut *connection)
		.await?;
	}

	for resource_type in [
		"workspace",
		"domain",
		"dnsRecord",
		"containerRegistryRepository",
		"managedDatabase",
		"deployment",
		"staticSite",
		"managedUrl",
		"secret",
		"staticSiteUpload",
		"region",
		"deploymentVolume",
		"ciRepo",
		"ciRunner",
	] {
		query!(
			r#"
			UPDATE
				resource_type
			SET
				name = CONCAT('test::', name)
			WHERE
				name = $1;
			"#,
			resource_type,
		)
		.execute(&mut *connection)
		.await?;

		query!(
			r#"
			UPDATE
				resource_type
			SET
				name = $1
			WHERE
				name = CONCAT('test::', $1);
			"#,
			resource_type,
		)
		.execute(&mut *connection)
		.await?;
	}

	Ok(())
}

async fn re_add_constraints(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE secret
		DROP CONSTRAINT secret_chk_name_is_trimmed;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment
		DROP CONSTRAINT deployment_chk_name_is_trimmed;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE static_site
		DROP CONSTRAINT static_site_chk_name_is_trimmed;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE secret
		ADD CONSTRAINT secret_chk_name_is_trimmed
		CHECK(name = TRIM(name));
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment
		ADD CONSTRAINT deployment_chk_name_is_trimmed
		CHECK(name = TRIM(name));
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE static_site
		ADD CONSTRAINT static_site_chk_name_is_trimmed
		CHECK(name = TRIM(name));
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
