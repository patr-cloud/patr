use api_models::{
	models::workspace::infrastructure::deployment::DeploymentStatus,
	utils::Uuid,
};

use crate::{
	models::db_mapping::DeploymentStaticSite,
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
			id UUID CONSTRAINT deployment_static_sites_pk PRIMARY KEY,
			name CITEXT NOT NULL
				CONSTRAINT deployment_static_sites_chk_name_is_trimmed CHECK(
					name = TRIM(name)
				),
			status DEPLOYMENT_STATUS NOT NULL DEFAULT 'created',
			workspace_id UUID NOT NULL,
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

	Ok(())
}

pub async fn create_static_site(
	connection: &mut <Database as sqlx::Database>::Connection,
	static_site_id: &Uuid,
	name: &str,
	workspace_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			deployment_static_sites
		VALUES
			($1, $2, 'created', $3);
		"#,
		static_site_id as _,
		name as _,
		workspace_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_static_site_by_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	static_site_id: &Uuid,
) -> Result<Option<DeploymentStaticSite>, sqlx::Error> {
	query_as!(
		DeploymentStaticSite,
		r#"
		SELECT
			id as "id: _",
			name::TEXT as "name!: _",
			status as "status: _",
			workspace_id as "workspace_id: _"
		FROM
			deployment_static_sites
		WHERE
			id = $1 AND
			status != 'deleted';
		"#,
		static_site_id as _,
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_static_site_by_name_in_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	name: &str,
	workspace_id: &Uuid,
) -> Result<Option<DeploymentStaticSite>, sqlx::Error> {
	query_as!(
		DeploymentStaticSite,
		r#"
		SELECT
			id as "id: _",
			name::TEXT as "name!: _",
			status as "status: _",
			workspace_id as "workspace_id: _"
		FROM
			deployment_static_sites
		WHERE
			name = $1 AND
			workspace_id = $2 AND
			status != 'deleted';
		"#,
		name as _,
		workspace_id as _,
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn update_static_site_status(
	connection: &mut <Database as sqlx::Database>::Connection,
	static_site_id: &Uuid,
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
		static_site_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_static_site_name(
	connection: &mut <Database as sqlx::Database>::Connection,
	static_site_id: &Uuid,
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
		static_site_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_static_sites_for_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<Vec<DeploymentStaticSite>, sqlx::Error> {
	query_as!(
		DeploymentStaticSite,
		r#"
		SELECT
			id as "id: _",
			name::TEXT as "name!: _",
			status as "status: _",
			workspace_id as "workspace_id: _"
		FROM
			deployment_static_sites
		WHERE
			workspace_id = $1 AND
			status != 'deleted';
		"#,
		workspace_id as _,
	)
	.fetch_all(&mut *connection)
	.await
}
