use api_models::utils::Uuid;
use chrono::{DateTime, Utc};

use crate::{query, query_as, Database};

pub struct Secret {
	pub id: Uuid,
	pub name: String,
	pub workspace_id: Uuid,
}

pub async fn initialize_secret_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Initializing secret tables");
	query!(
		r#"
		CREATE TABLE secret(
			id UUID CONSTRAINT secret_pk PRIMARY KEY,
			name CITEXT NOT NULL
				CONSTRAINT secret_chk_name_is_trimmed CHECK(name = TRIM(name)),
			workspace_id UUID NOT NULL,
			deleted TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn initialize_secret_constraints(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Finishing up secret tables initialization");
	// TODO create all the necessary indexes

	query!(
		r#"
		ALTER TABLE secret
		ADD CONSTRAINT secret_fk_id_workspace_id
		FOREIGN KEY(id, workspace_id) REFERENCES resource(id, owner_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

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

pub async fn get_secret_by_id(
	connection: &mut DatabaseConnection,
	secret_id: &Uuid,
) -> Result<Option<Secret>, sqlx::Error> {
	query_as!(
		Secret,
		r#"
		SELECT
			id as "id: _",
			name::TEXT as "name!: _",
			workspace_id as "workspace_id: _"
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
	connection: &mut DatabaseConnection,
	workspace_id: &Uuid,
) -> Result<Vec<Secret>, sqlx::Error> {
	query_as!(
		Secret,
		r#"
		SELECT
			id as "id: _",
			name::TEXT as "name!: _",
			workspace_id as "workspace_id: _"
		FROM
			secret
		WHERE
			workspace_id = $1 AND
			deleted IS NULL;
		"#,
		workspace_id as _,
	)
	.fetch_all(connection)
	.await
}

pub async fn create_new_secret_in_workspace(
	connection: &mut DatabaseConnection,
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
				workspace_id
			)
		VALUES
			($1, $2, $3);
		"#,
		secret_id as _,
		name as _,
		workspace_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_secret_name(
	connection: &mut DatabaseConnection,
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

pub async fn delete_secret(
	connection: &mut DatabaseConnection,
	secret_id: &Uuid,
	deletion_time: &DateTime<Utc>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			secret
		SET
			deleted = $2
		WHERE
			id = $1;
		"#,
		secret_id as _,
		deletion_time
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}
