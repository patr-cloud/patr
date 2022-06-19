use api_macros::{query, query_as};
use api_models::utils::Uuid;

use crate::Database;

pub struct Project {
	pub id: Uuid,
	pub workspace_id: Uuid,
	pub name: String,
	pub description: String,
}

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
			deleted BOOLEAN NOT NULL DEFAULT FALSE
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

	query!(
		r#"
		CREATE UNIQUE INDEX
			project_uq_idx_name_workspace_id
		ON
			project(name, workspace_id)
		WHERE
			deleted IS FALSE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn get_all_projects_in_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<Vec<Project>, sqlx::Error> {
	query_as!(
		Project,
		r#"
		SELECT
			id as "id: _",
			workspace_id as "workspace_id: _",
			name::TEXT as "name!: _",
			description as "description!: _"
		FROM
			project
		WHERE (
			workspace_id = $1
			AND deleted IS FALSE
		);
		"#,
		workspace_id as _,
	)
	.fetch_all(connection)
	.await
}

pub async fn get_project_by_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	project_id: &Uuid,
) -> Result<Option<Project>, sqlx::Error> {
	query_as!(
		Project,
		r#"
		SELECT
			id as "id: _",
			workspace_id as "workspace_id: _",
			name::TEXT as "name!: _",
			description as "description!: _"
		FROM
			project
		WHERE (
			id = $1
			AND deleted IS FALSE
		);
		"#,
		project_id as _,
	)
	.fetch_optional(connection)
	.await
}

pub async fn create_project(
	connection: &mut <Database as sqlx::Database>::Connection,
	resource_id: &Uuid,
	workspace_id: &Uuid,
	name: &str,
	description: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			project (
				id,
				workspace_id,
				name,
				description
			)
		VALUES
			($1, $2, $3, $4);
		"#,
		resource_id as _,
		workspace_id as _,
		name as _,
		description as _,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn update_project(
	connection: &mut <Database as sqlx::Database>::Connection,
	project_id: &Uuid,
	name: Option<&str>,
	description: Option<&str>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE project
		SET
			name = COALESCE($2, name),
			description = COALESCE($3, description)
		WHERE id = $1;
		"#,
		project_id as _,
		name as _,
		description as _,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn delete_project(
	connection: &mut <Database as sqlx::Database>::Connection,
	project_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			project
		SET
			deleted = TRUE
		WHERE
			id = $1;
		"#,
		project_id as _,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
