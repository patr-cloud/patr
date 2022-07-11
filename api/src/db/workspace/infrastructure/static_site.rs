use api_models::{
	models::workspace::infrastructure::deployment::DeploymentStatus,
	utils::{DateTime, Uuid},
};
use chrono::Utc;

use crate::{query, query_as, Database};

pub struct StaticSite {
	pub id: Uuid,
	pub name: String,
	pub status: DeploymentStatus,
	pub workspace_id: Uuid,
}

pub struct StaticSiteUploadHistory {
	pub id: Uuid,
	pub message: String,
	pub uploaded_by: Uuid,
	pub created: DateTime<Utc>,
}

pub async fn initialize_static_site_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing static site tables");
	query!(
		r#"
		CREATE TABLE static_site(
			id UUID CONSTRAINT static_site_pk PRIMARY KEY,
			name CITEXT NOT NULL
				CONSTRAINT static_site_chk_name_is_trimmed CHECK(
					name = TRIM(name)
				),
			status DEPLOYMENT_STATUS NOT NULL DEFAULT 'created',
			workspace_id UUID NOT NULL,
			CONSTRAINT static_site_uq_name_workspace_id
				UNIQUE(name, workspace_id),
			CONSTRAINT static_site_uq_id_workspace_id
				UNIQUE(id, workspace_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE static_site_upload_history(
			upload_id UUID CONSTRAINT static_site_upload_history_pk PRIMARY KEY,
			static_site_id UUID NOT NULL CONSTRAINT
				static_site_upload_history_fk_static_site_id
					REFERENCES static_site(id),
			message TEXT NOT NULL,
			uploaded_by UUID NOT NULL CONSTRAINT
				static_site_upload_history_fk_uploaded_by
					REFERENCES "user"(id),
			created TIMESTAMPTZ NOT NULL,
			CONSTRAINT static_site_upload_history_uq_upload_id_static_site_id
				UNIQUE(upload_id, static_site_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn initialize_static_site_post(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Finishing up static site tables initialization");

	query!(
		r#"
		ALTER TABLE static_site
		ADD CONSTRAINT static_site_fk_id_workspace_id
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
			static_site(
				id,
				name,
				status,
				workspace_id
			)
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
) -> Result<Option<StaticSite>, sqlx::Error> {
	query_as!(
		StaticSite,
		r#"
		SELECT
			id as "id: _",
			name::TEXT as "name!: _",
			status as "status: _",
			workspace_id as "workspace_id: _"
		FROM
			static_site
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
) -> Result<Option<StaticSite>, sqlx::Error> {
	query_as!(
		StaticSite,
		r#"
		SELECT
			id as "id: _",
			name::TEXT as "name!: _",
			status as "status: _",
			workspace_id as "workspace_id: _"
		FROM
			static_site
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
			static_site
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
			static_site
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
) -> Result<Vec<StaticSite>, sqlx::Error> {
	query_as!(
		StaticSite,
		r#"
		SELECT
			id as "id: _",
			name::TEXT as "name!: _",
			status as "status: _",
			workspace_id as "workspace_id: _"
		FROM
			static_site
		WHERE
			workspace_id = $1 AND
			status != 'deleted';
		"#,
		workspace_id as _,
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn generate_new_static_site_upload_id(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Uuid, sqlx::Error> {
	loop {
		let upload_id = Uuid::new_v4();

		let exists = query!(
			r#"
			SELECT
				*
			FROM
				static_site_upload_history
			WHERE
				upload_id = $1;
			"#,
			&upload_id as _,
		)
		.fetch_optional(&mut *connection)
		.await?
		.is_some();

		if !exists {
			break Ok(upload_id);
		}
	}
}

pub async fn create_static_site_upload_history(
	connection: &mut <Database as sqlx::Database>::Connection,
	upload_id: &Uuid,
	static_site_id: &Uuid,
	message: &str,
	uploaded_by: &Uuid,
	created: &DateTime<Utc>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			static_site_upload_history(
				upload_id,
				static_site_id,
				message,
				uploaded_by,
				created
			)
		VALUES
			($1, $2, $3, $4, $5);
		"#,
		upload_id as _,
		static_site_id as _,
		message as _,
		uploaded_by as _,
		created as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_static_site_upload_history(
	connection: &mut <Database as sqlx::Database>::Connection,
	static_site_id: &Uuid,
) -> Result<Vec<StaticSiteUploadHistory>, sqlx::Error> {
	query_as!(
		StaticSiteUploadHistory,
		r#"
		SELECT
			upload_id as "id: _",
			message,
			uploaded_by as "uploaded_by: _",
			created as "created: _"
		FROM
			static_site_upload_history
		WHERE
			static_site_id = $1;
		"#,
		static_site_id as _,
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn get_static_site_upload_history_by_upload_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	upload_id: &Uuid,
) -> Result<Option<StaticSiteUploadHistory>, sqlx::Error> {
	query_as!(
		StaticSiteUploadHistory,
		r#"
		SELECT
			upload_id as "id: _",
			message,
			uploaded_by as "uploaded_by: _",
			created as "created: _"
		FROM
			static_site_upload_history
		WHERE
			upload_id = $1;
		"#,
		upload_id as _,
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_latest_upload_for_static_site(
	connection: &mut <Database as sqlx::Database>::Connection,
	static_site_id: &Uuid,
) -> Result<Option<StaticSiteUploadHistory>, sqlx::Error> {
	query_as!(
		StaticSiteUploadHistory,
		r#"
		SELECT
			upload_id as "id: _",
			message,
			uploaded_by as "uploaded_by: _",
			created as "created: _"
		FROM
			static_site_upload_history
		WHERE
			static_site_id = $1
		ORDER BY
			created DESC
		LIMIT 1;
		"#,
		static_site_id as _,
	)
	.fetch_optional(&mut *connection)
	.await
}
