use sqlx::Row;

use crate::{
	migrate_query as query,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	refactor_resource_deletion(&mut *connection, config).await?;

	Ok(())
}

async fn refactor_resource_deletion(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	remove_resource_name_column(connection, config).await?;

	refactor_static_site_deletion(connection, config).await?;
	refactor_secret_deletion(connection, config).await?;
	refactor_docker_repository_deletion(connection, config).await?;
	refactor_database_deletion(connection, config).await?;
	refactor_deployment_deletion(connection, config).await?;
	refactor_workspace_deletion(connection, config).await?;
	refactor_domain_deletion(connection, config).await?;
	refactor_managed_url_deletion(connection, config).await?;

	Ok(())
}

async fn remove_resource_name_column(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE resource
		DROP COLUMN name;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn refactor_static_site_deletion(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE static_site
		ADD COLUMN deleted TIMESTAMPTZ;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE static_site
		DROP CONSTRAINT static_site_uq_name_workspace_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			static_site
		SET
			deleted = NOW()
		WHERE
			name LIKE CONCAT(
				'patr-deleted: ',
				REPLACE(id::TEXT, '-', ''),
				'@%'
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	loop {
		let static_sites_with_patr_deleted = query!(
			r#"
			SELECT
				COUNT(*) as "count"
			FROM
				static_site
			WHERE
				name LIKE CONCAT(
					'patr-deleted: ',
					REPLACE(id::TEXT, '-', ''),
					'@%'
				);
			"#
		)
		.fetch_one(&mut *connection)
		.await?
		.get::<i64, _>("count");

		if static_sites_with_patr_deleted <= 0 {
			break;
		}

		query!(
			r#"
			UPDATE
				static_site
			SET
				name = REPLACE(
					name,
					CONCAT(
						'patr-deleted: ',
						REPLACE(id::TEXT, '-', ''),
						'@'
					),
					''
				);
			"#
		)
		.execute(&mut *connection)
		.await?;
	}

	query!(
		r#"
		CREATE UNIQUE INDEX
			static_site_uq_name_workspace_id
		ON
			static_site(workspace_id, name)
		WHERE
			deleted IS NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn refactor_secret_deletion(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE secret
		ADD COLUMN deleted TIMESTAMPTZ;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE secret
		DROP CONSTRAINT secret_uq_workspace_id_name;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			secret
		SET
			deleted = NOW()
		WHERE
			name LIKE CONCAT(
				'patr-deleted: ',
				REPLACE(id::TEXT, '-', ''),
				'@%'
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	loop {
		let secrets_with_patr_deleted = query!(
			r#"
			SELECT
				COUNT(*) as "count"
			FROM
				secret
			WHERE
				name LIKE CONCAT(
					'patr-deleted: ',
					REPLACE(id::TEXT, '-', ''),
					'@%'
				);
			"#
		)
		.fetch_one(&mut *connection)
		.await?
		.get::<i64, _>("count");

		if secrets_with_patr_deleted <= 0 {
			break;
		}

		query!(
			r#"
			UPDATE
				secret
			SET
				name = REPLACE(
					name,
					CONCAT(
						'patr-deleted: ',
						REPLACE(id::TEXT, '-', ''),
						'@'
					),
					''
				);
			"#
		)
		.execute(&mut *connection)
		.await?;
	}

	query!(
		r#"
		CREATE UNIQUE INDEX
			secret_uq_workspace_id_name
		ON
			secret(workspace_id, name)
		WHERE
			deleted IS NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn refactor_docker_repository_deletion(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE docker_registry_repository
		ADD COLUMN deleted TIMESTAMPTZ;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE docker_registry_repository
		DROP CONSTRAINT docker_registry_repository_uq_workspace_id_name;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			docker_registry_repository
		SET
			deleted = NOW()
		WHERE
			name LIKE CONCAT(
				'patr-deleted: ',
				REPLACE(id::TEXT, '-', ''),
				'@%'
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	loop {
		let docker_repos_with_patr_deleted = query!(
			r#"
			SELECT
				COUNT(*) as "count"
			FROM
				docker_registry_repository
			WHERE
				name LIKE CONCAT(
					'patr-deleted: ',
					REPLACE(id::TEXT, '-', ''),
					'@%'
				);
			"#
		)
		.fetch_one(&mut *connection)
		.await?
		.get::<i64, _>("count");

		if docker_repos_with_patr_deleted <= 0 {
			break;
		}

		query!(
			r#"
			UPDATE
				docker_registry_repository
			SET
				name = REPLACE(
					name,
					CONCAT(
						'patr-deleted: ',
						REPLACE(id::TEXT, '-', ''),
						'@'
					),
					''
				);
			"#
		)
		.execute(&mut *connection)
		.await?;
	}

	query!(
		r#"
		CREATE UNIQUE INDEX
			docker_registry_repository_uq_workspace_id_name
		ON
			docker_registry_repository(workspace_id, name)
		WHERE
			deleted IS NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn refactor_database_deletion(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE managed_database
		ADD COLUMN deleted TIMESTAMPTZ;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE managed_database
		DROP CONSTRAINT managed_database_uq_name_workspace_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			managed_database
		SET
			deleted = NOW()
		WHERE
			name LIKE CONCAT(
				'patr-deleted: ',
				REPLACE(id::TEXT, '-', ''),
				'@%'
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	loop {
		let managed_databases_with_patr_deleted = query!(
			r#"
			SELECT
				COUNT(*) as "count"
			FROM
				managed_database
			WHERE
				name LIKE CONCAT(
					'patr-deleted: ',
					REPLACE(id::TEXT, '-', ''),
					'@%'
				);
			"#
		)
		.fetch_one(&mut *connection)
		.await?
		.get::<i64, _>("count");

		if managed_databases_with_patr_deleted <= 0 {
			break;
		}

		query!(
			r#"
			UPDATE
				managed_database
			SET
				name = REPLACE(
					name,
					CONCAT(
						'patr-deleted: ',
						REPLACE(id::TEXT, '-', ''),
						'@'
					),
					''
				);
			"#
		)
		.execute(&mut *connection)
		.await?;
	}

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

	Ok(())
}

async fn refactor_deployment_deletion(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE deployment
		ADD COLUMN deleted TIMESTAMPTZ;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment
		DROP CONSTRAINT deployment_uq_name_workspace_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			deployment
		SET
			deleted = NOW()
		WHERE
			name LIKE CONCAT(
				'patr-deleted: ',
				REPLACE(id::TEXT, '-', ''),
				'@%'
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	loop {
		let deployments_with_patr_deleted = query!(
			r#"
			SELECT
				COUNT(*) as "count"
			FROM
				deployment
			WHERE
				name LIKE CONCAT(
					'patr-deleted: ',
					REPLACE(id::TEXT, '-', ''),
					'@%'
				);
			"#
		)
		.fetch_one(&mut *connection)
		.await?
		.get::<i64, _>("count");

		if deployments_with_patr_deleted <= 0 {
			break;
		}

		query!(
			r#"
			UPDATE
				deployment
			SET
				name = REPLACE(
					name,
					CONCAT(
						'patr-deleted: ',
						REPLACE(id::TEXT, '-', ''),
						'@'
					),
					''
				);
			"#
		)
		.execute(&mut *connection)
		.await?;
	}

	query!(
		r#"
		CREATE UNIQUE INDEX
			deployment_uq_workspace_id_name
		ON
			deployment(workspace_id, name)
		WHERE
			deleted IS NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn refactor_workspace_deletion(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE workspace
		ADD COLUMN deleted TIMESTAMPTZ;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace
		DROP CONSTRAINT workspace_uq_name;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			workspace
		SET
			deleted = NOW()
		WHERE
			name LIKE CONCAT(
				'patr-deleted: ',
				REPLACE(id::TEXT, '-', ''),
				'@%'
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	loop {
		let workspaces_with_patr_deleted = query!(
			r#"
			SELECT
				COUNT(*) as "count"
			FROM
				workspace
			WHERE
				name LIKE CONCAT(
					'patr-deleted: ',
					REPLACE(id::TEXT, '-', ''),
					'@%'
				);
			"#
		)
		.fetch_one(&mut *connection)
		.await?
		.get::<i64, _>("count");

		if workspaces_with_patr_deleted <= 0 {
			break;
		}

		query!(
			r#"
			UPDATE
				workspace
			SET
				name = REPLACE(
					name,
					CONCAT(
						'patr-deleted: ',
						REPLACE(id::TEXT, '-', ''),
						'@'
					),
					''
				);
			"#
		)
		.execute(&mut *connection)
		.await?;
	}

	query!(
		r#"
		CREATE UNIQUE INDEX
			workspace_uq_name
		ON
			workspace(name)
		WHERE
			deleted IS NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn refactor_domain_deletion(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE domain
		ADD COLUMN deleted TIMESTAMPTZ;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE domain
		DROP CONSTRAINT domain_uq_name_tld;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE domain
		DROP CONSTRAINT domain_chk_name_is_valid;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			domain
		SET
			deleted = NOW()
		WHERE
			name LIKE CONCAT(
				'patr-deleted: ',
				REPLACE(id::TEXT, '-', ''),
				'@%'
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	loop {
		let domains_with_patr_deleted = query!(
			r#"
			SELECT
				COUNT(*) as "count"
			FROM
				domain
			WHERE
				name LIKE CONCAT(
					'patr-deleted: ',
					REPLACE(id::TEXT, '-', ''),
					'@%'
				);
			"#
		)
		.fetch_one(&mut *connection)
		.await?
		.get::<i64, _>("count");

		if domains_with_patr_deleted <= 0 {
			break;
		}

		query!(
			r#"
			UPDATE
				domain
			SET
				name = REPLACE(
					name,
					CONCAT(
						'patr-deleted: ',
						REPLACE(id::TEXT, '-', ''),
						'@'
					),
					''
				);
			"#
		)
		.execute(&mut *connection)
		.await?;
	}

	query!(
		r#"
		ALTER TABLE domain
		ADD CONSTRAINT domain_chk_name_is_valid CHECK(
			name ~ '^(([a-z0-9])|([a-z0-9][a-z0-9-]*[a-z0-9]))$'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE UNIQUE INDEX
			domain_uq_name_tld
		ON
			domain(name, tld)
		WHERE
			deleted IS NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE domain
		ADD CONSTRAINT domain_uq_id_type_deleted UNIQUE(id, type, deleted);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// TODO: migration for personal_domain and workspace_domain too

	query!(
		r#"
		ALTER TABLE personal_domain
			ADD COLUMN deleted TIMESTAMPTZ
				CONSTRAINT personal_domain_chk_deletion CHECK(
					deleted IS NULL
				),
			DROP CONSTRAINT personal_domain_fk_id_domain_type,
			ADD CONSTRAINT personal_domain_fk_id_domain_type_deleted
				FOREIGN KEY(id, domain_type, deleted) REFERENCES domain(id, type, deleted);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn refactor_managed_url_deletion(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE managed_url
		ADD COLUMN deleted TIMESTAMPTZ;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE managed_url
		DROP CONSTRAINT managed_url_uq_sub_domain_domain_id_path;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE managed_url
		DROP CONSTRAINT managed_url_chk_sub_domain_valid;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			managed_url
		SET
			deleted = NOW()
		WHERE
			name LIKE CONCAT(
				'patr-deleted: ',
				REPLACE(id::TEXT, '-', ''),
				'@%'
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	loop {
		let managed_urls_with_patr_deleted = query!(
			r#"
			SELECT
				COUNT(*) as "count"
			FROM
				managed_url
			WHERE
				name LIKE CONCAT(
					'patr-deleted: ',
					REPLACE(id::TEXT, '-', ''),
					'@%'
				);
			"#
		)
		.fetch_one(&mut *connection)
		.await?
		.get::<i64, _>("count");

		if managed_urls_with_patr_deleted <= 0 {
			break;
		}

		query!(
			r#"
			UPDATE
				managed_url
			SET
				name = REPLACE(
					name,
					CONCAT(
						'patr-deleted: ',
						REPLACE(id::TEXT, '-', ''),
						'@'
					),
					''
				);
			"#
		)
		.execute(&mut *connection)
		.await?;
	}

	query!(
		r#"
		ALTER TABLE managed_url
		ADD CONSTRAINT managed_url_chk_sub_domain_valid
			CHECK(
				sub_domain ~ '^(([a-z0-9_]|[a-z0-9_][a-z0-9_\-]*[a-z0-9_])\.)*([a-z0-9_]|[a-z0-9_][a-z0-9_\-]*[a-z0-9_])$' OR
				sub_domain = '@'
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE UNIQUE INDEX
			managed_url_uq_sub_domain_domain_id_path
		ON
			managed_url(sub_domain, domain_id, path)
		WHERE
			deleted IS NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
