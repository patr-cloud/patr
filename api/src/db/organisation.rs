use crate::{
	models::db_mapping::Domain, models::db_mapping::Organisation,
	models::db_mapping::Resource, query, query_as,
};

use sqlx::{pool::PoolConnection, MySqlConnection, Transaction};

pub async fn initialize_organisations_pre(
	transaction: &mut Transaction<PoolConnection<MySqlConnection>>,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing organisation tables");
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS organisation (
			id BINARY(16) PRIMARY KEY,
			name VARCHAR(100) UNIQUE NOT NULL,
			super_admin_id BINARY(16) NOT NULL,
			associated_resource_id BINARY(16) UNIQUE,
			active BOOL NOT NULL DEFAULT FALSE,
			created BIGINT UNSIGNED NOT NULL,
			FOREIGN KEY(super_admin_id) REFERENCES user(id)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	Ok(())
}

pub async fn initialize_organisations_post(
	transaction: &mut Transaction<PoolConnection<MySqlConnection>>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TABLE organisation
		ADD CONSTRAINT
		FOREIGN KEY(associated_resource_id) REFERENCES resource(id);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS domain (
			id BINARY(16) PRIMARY KEY,
			name VARCHAR(100) UNIQUE NOT NULL,
			is_verified BOOL NOT NULL DEFAULT FALSE,
			FOREIGN KEY(owner_organisation_id) REFERENCES organisation(id),
			FOREIGN KEY(id) REFERENCES resource(id)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	Ok(())
}

pub async fn create_organisation(
	connection: &mut Transaction<PoolConnection<MySqlConnection>>,
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
			(?, ?, ?, NULL, ?, ?);
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

pub async fn set_resource_id_for_organisation(
	connection: &mut Transaction<PoolConnection<MySqlConnection>>,
	organisation_id: &[u8],
	resource_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			organisation
		SET
			associated_resource_id = ?
		WHERE
			id = ?;
		"#,
		resource_id,
		organisation_id
	)
	.execute(connection)
	.await?;

	Ok(())
}

pub async fn add_domain_to_organisation(
	connection: &mut Transaction<PoolConnection<MySqlConnection>>,
	domain_id: &[u8],
	domain_name: &str,
	organisation_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			domain
		VALUES
			(?, ?, ?, FALSE);
		"#,
		domain_id,
		domain_name,
		organisation_id
	)
	.execute(connection)
	.await?;

	Ok(())
}

pub async fn get_organisation_info(
	connection: &mut Transaction<PoolConnection<MySqlConnection>>,
	organisation_id: &[u8],
) -> Result<Option<Organisation>, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT
			*
		FROM
			organisation
		WHERE
			id = ?;
		"#,
		organisation_id
	)
	.fetch_all(connection)
	.await?;

	if rows.is_empty() {
		return Ok(None);
	}
	let row = rows.into_iter().next().unwrap();

	Ok(Some(Organisation {
		id: row.id,
		name: row.name,
		super_admin_id: row.super_admin_id,
		associated_resource_id: row.associated_resource_id.unwrap(),
		active: row.active > 0,
		created: row.created,
	}))
}

pub async fn get_resource_for_organisation(
	connection: &mut Transaction<PoolConnection<MySqlConnection>>,
	organisation_id: &[u8],
) -> Result<Option<Resource>, sqlx::Error> {
	let rows = query_as!(
		Resource,
		r#"
		SELECT
			resource.*
		FROM
			organisation, resource
		WHERE
			organisation.id = ? AND
			organisation.associated_resource_id = resource.id;
		"#,
		organisation_id
	)
	.fetch_all(connection)
	.await?;

	Ok(rows.into_iter().next())
}

pub async fn get_domains_for_organisation(
	connection: &mut Transaction<PoolConnection<MySqlConnection>>,
	organisation_id: &[u8],
) -> Result<Vec<Domain>, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT
			*
		FROM
			domain
		WHERE
			owner_organisation_id = ?;
		"#,
		organisation_id
	)
	.fetch_all(connection)
	.await?;

	let mut domains = Vec::with_capacity(rows.len());

	for row in rows {
		domains.push(Domain {
			id: row.id,
			name: row.name,
			owner_organisation_id: row.owner_organisation_id,
			is_verified: row.is_verified > 0,
		});
	}

	Ok(domains)
}
