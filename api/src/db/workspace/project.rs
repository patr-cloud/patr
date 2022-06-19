use api_macros::query;

use crate::Database;

pub async fn initialize_project_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing project tables");
	// create project table
	query!(
		r#"
		CREATE TABLE project (
			id UUID
				CONSTRAINT project_pk PRIMARY KEY,
			workspace_id UUID NOT NULL
				CONSTRAINT project_fk_workspace_id REFERENCES workspace(id),
			name CITEXT NOT NULL,
			description VARCHAR(500) NOT NULL,

			CONSTRAINT project_uq_workspace_id_name
				UNIQUE (workspace_id, name)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn initialize_project_post(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Finishing up project tables initialization");

	// add constraints for project table
	query!(
		r#"
		ALTER TABLE project
			ADD CONSTRAINT project_fk_id_workspace_id
				FOREIGN KEY(id, workspace_id)
					REFERENCES resource(id, owner_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
