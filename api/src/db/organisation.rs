use crate::{
	models::db_mapping::{Domain, Organisation},
	query,
	query_as,
};

use sqlx::{MySql, Transaction};

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

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS domain (
			id BINARY(16) PRIMARY KEY,
			name VARCHAR(100) UNIQUE NOT NULL,
			is_verified BOOL NOT NULL DEFAULT FALSE
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

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

	query!(
		r#"
		ALTER TABLE domain
		ADD CONSTRAINT
		FOREIGN KEY(id) REFERENCES resource(id);
		"#
	)
	.execute(&mut *transaction)
	.await?;

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

pub async fn add_domain_to_organisation(
	connection: &mut Transaction<'_, MySql>,
	domain_id: &[u8],
	domain_name: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			domain
		VALUES
			(?, ?, FALSE);
		"#,
		domain_id,
		domain_name,
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
			"active: bool",
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

pub async fn get_domains_for_organisation(
	connection: &mut Transaction<'_, MySql>,
	organisation_id: &[u8],
) -> Result<Vec<Domain>, sqlx::Error> {
	let rows = query_as!(
		Domain,
		r#"
		SELECT
			domain.id,
			domain.name,
			"is_verified: bool"
		FROM
			domain, resource
		WHERE
			resource.owner_id = ? AND
			resource.id = domain.id;
		"#,
		organisation_id
	)
	.fetch_all(connection)
	.await?;

	Ok(rows)
}

pub async fn get_all_unverified_domains(
	connection: &mut Transaction<'_, MySql>,
) -> Result<Vec<Domain>, sqlx::Error> {
	let rows = query_as!(
		Domain,
		r#"
		SELECT
			id,
			name,
			"is_verified: bool"
		FROM
			domain
		WHERE
			is_verified = FALSE;
		"#
	)
	.fetch_all(connection)
	.await?;

	Ok(rows)
}

pub async fn set_domain_as_verified(
	connection: &mut Transaction<'_, MySql>,
	domain_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			domain
		SET
			is_verified = TRUE
		WHERE
			id = ?;
		"#,
		domain_id
	)
	.execute(connection)
	.await?;

	Ok(())
}

pub async fn get_all_verified_domains(
	connection: &mut Transaction<'_, MySql>,
) -> Result<Vec<Domain>, sqlx::Error> {
	let rows = query_as!(
		Domain,
		r#"
		SELECT
			id,
			name,
			"is_verified: bool"
		FROM
			domain
		WHERE
			is_verified = TRUE;
		"#
	)
	.fetch_all(connection)
	.await?;

	Ok(rows)
}

pub async fn set_domain_as_unverified(
	connection: &mut Transaction<'_, MySql>,
	domain_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			domain
		SET
			is_verified = FALSE
		WHERE
			id = ?;
		"#,
		domain_id
	)
	.execute(connection)
	.await?;

	Ok(())
}

pub async fn get_notification_email_for_domain(
	connection: &mut Transaction<'_, MySql>,
	domain_id: &[u8],
) -> Result<Option<String>, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT
			user.*
		FROM
			domain, resource, organisation, user
		WHERE
			domain.id = ? AND
			domain.id = resource.id AND
			resource.owner_id = organisation.id AND
			organisation.super_admin_id = user.id;
		"#,
		domain_id
	)
	.fetch_all(connection)
	.await?;

	if rows.is_empty() {
		return Ok(None);
	}
	let row = rows.into_iter().next().unwrap();

	Ok(Some(row.backup_email))
}

pub async fn delete_domain_from_organisation(
	connection: &mut Transaction<'_, MySql>,
	domain_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			domain
		WHERE
			id = ?;
		"#,
		domain_id
	)
	.execute(connection)
	.await?;

	Ok(())
}

pub async fn get_domain_by_id(
	connection: &mut Transaction<'_, MySql>,
	domain_id: &[u8],
) -> Result<Option<Domain>, sqlx::Error> {
	let rows = query_as!(
		Domain,
		r#"
		SELECT
			id,
			name,
			"is_verified: bool"
		FROM
			domain
		WHERE
			id = ?
		"#,
		domain_id
	)
	.fetch_all(connection)
	.await?;

	Ok(rows.into_iter().next())
}
