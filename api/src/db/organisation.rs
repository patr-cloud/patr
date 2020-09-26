use crate::query;

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
			active BOOL NOT NULL DEFAULT FALSE,
			created BIGINT UNSIGNED NOT NULL,
			FOREIGN KEY(super_admin_id) REFERENCES user(id)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS domain (
			id BINARY(16) PRIMARY KEY,
			name VARCHAR(100) UNIQUE NOT NULL,
			owner_organisation_id BINARY(16) NOT NULL,
			is_verified BOOL NOT NULL DEFAULT FALSE,
			FOREIGN KEY(owner_organisation_id) REFERENCES organisation(id)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	Ok(())
}

pub async fn initialize_organisations_post(
	_transaction: &mut Transaction<PoolConnection<MySqlConnection>>,
) -> Result<(), sqlx::Error> {
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
			organisation (
				id,
				name,
				super_admin_id,
				active,
				created
			)
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

pub async fn add_domain_to_organisation(
	connection: &mut Transaction<PoolConnection<MySqlConnection>>,
	domain_id: &[u8],
	domain_name: &str,
	organisation_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			domain (
				id,
				name,
				owner_organisation_id
			)
		VALUES
			(?, ?, ?);
		"#,
		domain_id,
		domain_name,
		organisation_id
	)
	.execute(connection)
	.await?;

	Ok(())
}
