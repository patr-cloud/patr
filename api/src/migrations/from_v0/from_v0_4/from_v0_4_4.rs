use crate::{migrate_query as query, utils::settings::Settings, Database};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			deployment
		SET
			name = TRIM(name);
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
		UPDATE
			managed_database
		SET
			name = TRIM(name),
			db_name = TRIM(db_name);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE managed_database
		ADD CONSTRAINT managed_database_chk_name_is_trimmed
		CHECK(name = TRIM(name));
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE managed_database
		ADD CONSTRAINT managed_database_chk_db_name_is_trimmed
		CHECK(db_name = TRIM(db_name));
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			deployment_static_sites
		SET
			name = TRIM(name);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment_static_sites
		ADD CONSTRAINT deployment_static_sites_chk_name_is_trimmed
		CHECK(name = TRIM(name));
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		SET CONSTRAINTS ALL DEFERRED;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			deployment
		SET
			domain_name = CONCAT(
				'deleted.patr.cloud.',
				ENCODE(id, 'hex'),
				'.',
				REPLACE(
					domain_name,
					CONCAT(
						'deleted.patr.cloud.',
						ENCODE(id, 'hex')
					),
					''
				)
			)
		WHERE
			domain_name NOT LIKE CONCAT(
				'deleted.patr.cloud.',
				ENCODE(id, 'hex'),
				'.%'
			) AND
			status = 'deleted';
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			deployed_domain
		SET
			domain_name = deployment.domain_name
		FROM
			deployment
		WHERE
			deployed_domain.deployment_id = deployment.id AND
			deployment.status = 'deleted';
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		SET CONSTRAINTS ALL IMMEDIATE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
