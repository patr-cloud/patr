use api_models::utils::Uuid;

use crate::{query, query_as, Database};

pub struct Secret {
	pub id: Uuid,
	pub name: String,
	pub workspace_id: Uuid,
	pub deployment_id: Option<Uuid>,
}

pub async fn initialize_secret_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing secret tables");
	query!(
		r#"
		CREATE TABLE secret(
			id UUID CONSTRAINT secret_pk PRIMARY KEY,
			name CITEXT NOT NULL
				CONSTRAINT secret_chk_name_is_trimmed CHECK(name = TRIM(name)),
			workspace_id UUID NOT NULL,
			deployment_id UUID, /* For deployment specific secrets */
			billable_service_id UUID NOT NULL,
			CONSTRAINT secret_uq_workspace_id_name UNIQUE(workspace_id, name)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn initialize_secret_post(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Finishing up secret tables initialization");
	// TODO create all the necessary indexes

	query!(
		r#"
		ALTER TABLE secret
			ADD CONSTRAINT secret_fk_id_workspace_id
				FOREIGN KEY(id, workspace_id) 
					REFERENCES resource(id, owner_id),
			ADD CONSTRAINT secret_fk_deployment_id_workspace_id
				FOREIGN KEY(deployment_id, workspace_id)
					REFERENCES deployment(id, workspace_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn get_secret_by_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	secret_id: &Uuid,
) -> Result<Option<Secret>, sqlx::Error> {
	query_as!(
		Secret,
		r#"
		SELECT
			id as "id: _",
			name::TEXT as "name!: _",
			workspace_id as "workspace_id: _",
			deployment_id as "deployment_id: _"
		FROM
			secret
		WHERE
			id = $1;
		"#,
		secret_id as _
	)
	.fetch_optional(connection)
	.await
}

pub async fn get_all_secrets_in_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<Vec<Secret>, sqlx::Error> {
	query_as!(
		Secret,
		r#"
		SELECT
			id as "id: _",
			name::TEXT as "name!: _",
			workspace_id as "workspace_id: _",
			deployment_id as "deployment_id: _"
		FROM
			secret
		WHERE
			workspace_id = $1 AND
			name NOT LIKE CONCAT(
				'patr-deleted: ',
				REPLACE(id::TEXT, '-', ''),
				'@%'
			);
		"#,
		workspace_id as _,
	)
	.fetch_all(connection)
	.await
}

pub async fn create_new_secret_in_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	secret_id: &Uuid,
	name: &str,
	workspace_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			secret(
				id,
				name,
				workspace_id,
				deployment_id
			)
		VALUES
			($1, $2, $3, NULL);
		"#,
		secret_id as _,
		name as _,
		workspace_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

#[allow(dead_code)]
pub async fn create_new_secret_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	secret_id: &Uuid,
	name: &str,
	workspace_id: &Uuid,
	deployment_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			secret(
				id,
				name,
				workspace_id,
				deployment_id
			)
		VALUES
			($1, $2, $3, $4);
		"#,
		secret_id as _,
		name as _,
		workspace_id as _,
		deployment_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_secret_name(
	connection: &mut <Database as sqlx::Database>::Connection,
	secret_id: &Uuid,
	name: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			secret
		SET
			name = $1
		WHERE
			id = $2;
		"#,
		name as _,
		secret_id as _
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}
