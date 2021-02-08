use crate::{models::db_mapping::Organisation, query, query_as};

use sqlx::{MySql, Transaction};

mod application;
mod domain;
mod drive;
mod pi_tunnel;

pub use application::*;
pub use domain::*;
pub use drive::*;
pub use pi_tunnel::*;

pub async fn initialize_organisations_pre(
	transaction: &mut Transaction<'_, MySql>,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing organisation tables");
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS organisation (
			id BINARY(16) PRIMARY KEY,
			name VARCHAR(100) UNIQUE NOT NULL,
			super_admin_id BINARY(16) NOT NULL,
			active BOOL NOT NULL DEFAULT FALSE,
			created BIGINT UNSIGNED NOT NULL,
			FOREIGN KEY(super_admin_id) REFERENCES user(id)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	application::initialize_application_pre(&mut *transaction).await?;
	domain::initialize_domain_pre(&mut *transaction).await?;
	drive::initialize_drive_pre(&mut *transaction).await?;
	pi_tunnel::initialize_pi_tunnel_pre(&mut *transaction).await?;

	Ok(())
}

pub async fn initialize_organisations_post(
	transaction: &mut Transaction<'_, MySql>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TABLE organisation
		ADD CONSTRAINT
		FOREIGN KEY(id) REFERENCES resource(id);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	application::initialize_application_post(&mut *transaction).await?;
	domain::initialize_domain_post(&mut *transaction).await?;
	drive::initialize_drive_post(&mut *transaction).await?;
	// pi_tunnel::initialize_pi_tunnel_post(&mut *transaction).await?;

	Ok(())
}

pub async fn create_organisation(
	connection: &mut Transaction<'_, MySql>,
	organisation_id: &[u8],
	name: &str,
	super_admin_id: &[u8],
	created: u64,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			organisation
		VALUES
			(?, ?, ?, ?, ?);
		"#,
		organisation_id,
		name,
		super_admin_id,
		true,
		created,
	)
	.execute(connection)
	.await?;

	Ok(())
}

pub async fn get_organisation_info(
	connection: &mut Transaction<'_, MySql>,
	organisation_id: &[u8],
) -> Result<Option<Organisation>, sqlx::Error> {
	let rows = query_as!(
		Organisation,
		r#"
		SELECT
			id,
			name,
			super_admin_id,
			active as `active: bool`,
			created
		FROM
			organisation
		WHERE
			id = ?;
		"#,
		organisation_id
	)
	.fetch_all(connection)
	.await?;

	Ok(rows.into_iter().next())
}

pub async fn get_organisation_by_name(
	connection: &mut Transaction<'_, MySql>,
	name: &str,
) -> Result<Option<Organisation>, sqlx::Error> {
	let rows = query_as!(
		Organisation,
		r#"
		SELECT
			id,
			name,
			super_admin_id,
			active as `active: bool`,
			created
		FROM
			organisation
		WHERE
			name = ?;
		"#,
		name
	)
	.fetch_all(connection)
	.await?;

	Ok(rows.into_iter().next())
}

pub async fn update_organisation_name(
	connection: &mut Transaction<'_, MySql>,
	organisation_id: &[u8],
	name: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			organisation
		SET
			name = ?
		WHERE
			id = ?;
		"#,
		name,
		organisation_id,
	)
	.execute(connection)
	.await?;

	Ok(())
}
