use crate::{
	models::db_mapping::{DeploymentStaticSite, DeploymentStatus},
	query,
	query_as,
	Database,
};

pub async fn initialize_static_sites_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing static sites tables");

	query!(
		r#"
		CREATE TABLE deployment_static_sites(
			id BYTEA CONSTRAINT deployment_static_sites_pk PRIMARY KEY,
			name CITEXT NOT NULL
				CONSTRAINT deployment_static_sites_chk_name_is_trimmed CHECK(
					name = TRIM(name)
				),
			status DEPLOYMENT_STATUS NOT NULL DEFAULT 'created',
			domain_name VARCHAR(255)
				CONSTRAINT deployment_static_sites_uq_domain_name UNIQUE
				CONSTRAINT
					deployment_static_sites_chk_domain_name_is_lower_case
						CHECK(domain_name = LOWER(domain_name)),
			workspace_id BYTEA NOT NULL,
			CONSTRAINT deployment_static_sites_uq_name_workspace_id
				UNIQUE(name, workspace_id),
			CONSTRAINT deployment_static_sites_uq_id_domain_name
				UNIQUE(id, domain_name)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn initialize_static_sites_post(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Finishing up static sites tables initialization");

	query!(
		r#"
		ALTER TABLE deployment_static_sites
		ADD CONSTRAINT deployment_static_sites_fk_id_workspace_id
		FOREIGN KEY(id, workspace_id) REFERENCES resource(id, owner_id);
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

	Ok(())
}

pub async fn create_static_site(
	connection: &mut <Database as sqlx::Database>::Connection,
	static_site_id: &[u8],
	name: &str,
	domain_name: Option<&str>,
	workspace_id: &[u8],
) -> Result<(), sqlx::Error> {
	if let Some(domain) = domain_name {
		query!(
			r#"
			INSERT INTO
				deployment_static_sites
			VALUES
				($1, $2, 'created', $3, $4);
			"#,
			static_site_id,
			name as _,
			domain,
			workspace_id
		)
		.execute(&mut *connection)
		.await
		.map(|_| ())
	} else {
		query!(
			r#"
			INSERT INTO
				deployment_static_sites
			VALUES
				($1, $2, 'created', NULL, $3);
			"#,
			static_site_id,
			name as _,
			workspace_id
		)
		.execute(&mut *connection)
		.await
		.map(|_| ())
	}
}

pub async fn get_static_site_by_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	static_site_id: &[u8],
) -> Result<Option<DeploymentStaticSite>, sqlx::Error> {
	query_as!(
		DeploymentStaticSite,
		r#"
		SELECT
			id,
			name as "name: _",
			status as "status: _",
			domain_name,
			workspace_id
		FROM
			deployment_static_sites
		WHERE
			id = $1 AND
			status != 'deleted';
		"#,
		static_site_id
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_static_site_by_name_in_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	name: &str,
	workspace_id: &[u8],
) -> Result<Option<DeploymentStaticSite>, sqlx::Error> {
	query_as!(
		DeploymentStaticSite,
		r#"
		SELECT
			id,
			name as "name: _",
			status as "status: _",
			domain_name,
			workspace_id
		FROM
			deployment_static_sites
		WHERE
			name = $1 AND
			workspace_id = $2 AND
			status != 'deleted';
		"#,
		name as _,
		workspace_id,
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn update_static_site_status(
	connection: &mut <Database as sqlx::Database>::Connection,
	static_site_id: &[u8],
	status: &DeploymentStatus,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			deployment_static_sites
		SET
			status = $1
		WHERE
			id = $2;
		"#,
		status as _,
		static_site_id
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_static_site_name(
	connection: &mut <Database as sqlx::Database>::Connection,
	static_site_id: &[u8],
	name: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			deployment_static_sites
		SET
			name = $1
		WHERE
			id = $2;
		"#,
		name as _,
		static_site_id
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_static_sites_for_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &[u8],
) -> Result<Vec<DeploymentStaticSite>, sqlx::Error> {
	query_as!(
		DeploymentStaticSite,
		r#"
		SELECT
			id,
			name as "name: _",
			status as "status: _",
			domain_name,
			workspace_id
		FROM
			deployment_static_sites
		WHERE
			workspace_id = $1 AND
			status != 'deleted';
		"#,
		workspace_id
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn set_domain_name_for_static_site(
	connection: &mut <Database as sqlx::Database>::Connection,
	static_site_id: &[u8],
	domain_name: Option<&str>,
) -> Result<(), sqlx::Error> {
	if let Some(domain_name) = domain_name {
		query!(
			r#"
			INSERT INTO
				deployed_domain
			VALUES
				(NULL, $1, $2)
			ON CONFLICT(static_site_id) DO UPDATE SET
				domain_name = EXCLUDED.domain_name;
			"#,
			static_site_id,
			domain_name,
		)
		.execute(&mut *connection)
		.await?;

		query!(
			r#"
			UPDATE
				deployment_static_sites
			SET
				domain_name = $1
			WHERE
				id = $2;
			"#,
			domain_name,
			static_site_id,
		)
		.execute(&mut *connection)
		.await
		.map(|_| ())
	} else {
		query!(
			r#"
			DELETE FROM
				deployed_domain
			WHERE
				static_site_id = $1;
			"#,
			static_site_id,
		)
		.execute(&mut *connection)
		.await?;

		query!(
			r#"
			UPDATE
				deployment_static_sites
			SET
				domain_name = NULL
			WHERE
				id = $1;
			"#,
			static_site_id,
		)
		.execute(&mut *connection)
		.await
		.map(|_| ())
	}
}
