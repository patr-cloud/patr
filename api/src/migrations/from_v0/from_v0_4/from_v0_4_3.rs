use crate::{
	migrate_query as query,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		UPDATE
			deployment
		SET
			domain_name = CONCAT('deleted.patr.cloud.', ENCODE(id, 'hex'), domain_name)
		WHERE
			domain_name IS NOT NULL AND
			status = 'deleted';
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			deployment_static_sites
		SET
			domain_name = CONCAT('deleted.patr.cloud.', ENCODE(id, 'hex'), domain_name)
		WHERE
			domain_name IS NOT NULL AND
			status = 'deleted';
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE deployed_domain(
			deployment_id BYTEA
				CONSTRAINT deployed_domain_uq_deployment_id UNIQUE,
			static_site_id BYTEA
				CONSTRAINT deployed_domain_uq_static_site_id UNIQUE,
			domain_name VARCHAR(255) NOT NULL
				CONSTRAINT deployed_domain_uq_domain_name UNIQUE
				CONSTRAINT deployment_chk_domain_name_is_lower_case CHECK(
					domain_name = LOWER(domain_name)
				),
			CONSTRAINT deployed_domain_uq_deployment_id_domain_name UNIQUE (deployment_id, domain_name),
			CONSTRAINT deployed_domain_uq_static_site_id_domain_name UNIQUE (static_site_id, domain_name),
			CONSTRAINT deployed_domain_chk_id_domain_is_valid CHECK(
				(
					deployment_id IS NULL AND
					static_site_id IS NOT NULL
				) OR
				(
					deployment_id IS NOT NULL AND
					static_site_id IS NULL
				)
			)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		INSERT INTO
			deployed_domain (deployment_id, domain_name)
			SELECT
				deployment.id,
				deployment.domain_name
			FROM
				deployment
			WHERE 
				domain_name IS NOT NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		INSERT INTO
			deployed_domain (static_site_id, domain_name)
			SELECT
				deployment_static_sites.id,
				deployment_static_sites.domain_name
			FROM
				deployment_static_sites				
			WHERE 
				domain_name IS NOT NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment
		ADD CONSTRAINT deployment_uq_id_domain_name
		UNIQUE(id, domain_name);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment
		ADD CONSTRAINT deployment_fk_id_domain_name
		FOREIGN KEY(id, domain_name) REFERENCES deployed_domain(deployment_id, domain_name)
		DEFERRABLE INITIALLY IMMEDIATE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment_static_sites
		ADD CONSTRAINT deployment_static_sites_uq_id_domain_name
		UNIQUE(id, domain_name);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment_static_sites
		ADD CONSTRAINT deployment_static_sites_fk_id_domain_name
		FOREIGN KEY(id, domain_name) REFERENCES deployed_domain(static_site_id, domain_name)
		DEFERRABLE INITIALLY IMMEDIATE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment_static_sites 
		RENAME CONSTRAINT deployment_static_sites_pk_uq_domain_name 
		TO deployment_static_sites_uq_domain_name;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment_static_sites 
		RENAME CONSTRAINT deployment_static_sites_pk_chk_domain_name_is_lower_case 
		TO deployment_static_sites_chk_domain_name_is_lower_case;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployed_domain
		ADD CONSTRAINT deployed_domain_fk_deployment_id_domain_name
		FOREIGN KEY(deployment_id, domain_name) REFERENCES deployment(id, domain_name)
		DEFERRABLE INITIALLY IMMEDIATE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployed_domain
		ADD CONSTRAINT deployed_domain_fk_static_site_id_domain_name
		FOREIGN KEY(static_site_id, domain_name) REFERENCES deployment_static_sites(id, domain_name)
		DEFERRABLE INITIALLY IMMEDIATE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
