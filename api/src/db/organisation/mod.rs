use crate::{models::db_mapping::Organisation, query, query_as, Database};

mod application;
mod deployment;
mod domain;
mod drive;
mod managed_database;
mod portus;

pub use application::*;
pub use deployment::*;
pub use domain::*;
pub use drive::*;
pub use managed_database::*;
pub use portus::*;

pub async fn initialize_organisations_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing organisation tables");
	query!(
		r#"
		CREATE TABLE organisation(
			id BYTEA
				CONSTRAINT organisation_pk PRIMARY KEY,
			name VARCHAR(100) NOT NULL
				CONSTRAINT organisation_uq_name UNIQUE,
			super_admin_id BYTEA NOT NULL
				CONSTRAINT organisation_super_admin_id_fk_user_id
					REFERENCES "user"(id),
			active BOOLEAN NOT NULL DEFAULT FALSE
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			organisation_idx_super_admin_id
		ON
			organisation
		(super_admin_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			organisation_idx_active
		ON
			organisation
		(active);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// Ref: https://www.postgresql.org/docs/13/datatype-enum.html
	query!(
		r#"
		CREATE TYPE RESOURCE_OWNER_TYPE AS ENUM(
			'personal',
			'organisation'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	application::initialize_application_pre(connection).await?;
	domain::initialize_domain_pre(connection).await?;
	drive::initialize_drive_pre(connection).await?;
	portus::initialize_portus_pre(connection).await?;
	deployment::initialize_deployment_pre(connection).await?;
	managed_database::initialize_managed_database_pre(connection).await?;

	Ok(())
}

pub async fn initialize_organisations_post(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Finishing up organisation tables initialization");
	query!(
		r#"
		ALTER TABLE organisation
		ADD CONSTRAINT organisation_fk_id
		FOREIGN KEY(id) REFERENCES resource(id)
		DEFERRABLE INITIALLY IMMEDIATE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	application::initialize_application_post(connection).await?;
	domain::initialize_domain_post(connection).await?;
	drive::initialize_drive_post(connection).await?;
	portus::initialize_portus_post(connection).await?;
	deployment::initialize_deployment_post(connection).await?;
	managed_database::initialize_deployment_post(connection).await?;

	Ok(())
}

pub async fn create_organisation(
	connection: &mut <Database as sqlx::Database>::Connection,
	organisation_id: &[u8],
	name: &str,
	super_admin_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			organisation
		VALUES
			($1, $2, $3, $4);
		"#,
		organisation_id,
		name,
		super_admin_id,
		true,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn get_organisation_info(
	connection: &mut <Database as sqlx::Database>::Connection,
	organisation_id: &[u8],
) -> Result<Option<Organisation>, sqlx::Error> {
	let row = query_as!(
		Organisation,
		r#"
		SELECT
			*
		FROM
			organisation
		WHERE
			id = $1;
		"#,
		organisation_id
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.next();

	Ok(row)
}

pub async fn get_organisation_by_name(
	connection: &mut <Database as sqlx::Database>::Connection,
	name: &str,
) -> Result<Option<Organisation>, sqlx::Error> {
	let row = query_as!(
		Organisation,
		r#"
		SELECT
			*
		FROM
			organisation
		WHERE
			name = $1;
		"#,
		name
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.next();

	Ok(row)
}

pub async fn update_organisation_name(
	connection: &mut <Database as sqlx::Database>::Connection,
	organisation_id: &[u8],
	name: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			organisation
		SET
			name = $1
		WHERE
			id = $2;
		"#,
		name,
		organisation_id,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
