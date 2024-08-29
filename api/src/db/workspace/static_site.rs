use crate::prelude::*;

/// Initializes the static site tables
#[instrument(skip(connection))]
pub async fn initialize_static_site_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up static site tables");

	query!(
		r#"
		CREATE TABLE static_site(
			id UUID NOT NULL,
			name CITEXT NOT NULL,
			status DEPLOYMENT_STATUS NOT NULL DEFAULT 'created',
			workspace_id UUID NOT NULL,
			current_live_upload UUID,
			deleted TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE static_site_upload_history(
			upload_id UUID NOT NULL,
			static_site_id UUID NOT NULL,
			message TEXT NOT NULL,
			uploaded_by UUID NOT NULL,
			created TIMESTAMPTZ NOT NULL,
			processed TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes the static site indices
#[instrument(skip(connection))]
pub async fn initialize_static_site_indices(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up static site indices");
	query!(
		r#"
		ALTER TABLE static_site
			ADD CONSTRAINT static_site_pk PRIMARY KEY(id),
			ADD CONSTRAINT static_site_uq_id_workspace_id UNIQUE(id, workspace_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE static_site_upload_history
			ADD CONSTRAINT static_site_upload_history_pk PRIMARY KEY(upload_id),
			ADD CONSTRAINT static_site_upload_history_uq_upload_id_static_site_id
				UNIQUE(upload_id, static_site_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

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

/// Initializes the static site constraints
#[instrument(skip(connection))]
pub async fn initialize_static_site_constraints(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up static site constraints");
	query!(
		r#"
		ALTER TABLE static_site
			ADD CONSTRAINT static_site_chk_name_is_trimmed CHECK(
				name = TRIM(name)
			),
			ADD CONSTRAINT static_site_fk_id_workspace_id_deleted
				FOREIGN KEY(id, workspace_id, deleted)
					REFERENCES resource(id, owner_id, deleted)
					DEFERRABLE INITIALLY IMMEDIATE,
			ADD CONSTRAINT static_site_fk_current_live_upload
				FOREIGN KEY(id, current_live_upload)
					REFERENCES static_site_upload_history(static_site_id, upload_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE static_site_upload_history
			ADD CONSTRAINT static_site_upload_history_fk_static_site_id
				FOREIGN KEY(static_site_id) REFERENCES static_site(id),
			ADD CONSTRAINT static_site_upload_history_fk_uploaded_by
				FOREIGN KEY(uploaded_by) REFERENCES "user"(id),
			ADD CONSTRAINT static_site_upload_history_fk_upload_id_resource_id
				FOREIGN KEY(upload_id) REFERENCES resource(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
