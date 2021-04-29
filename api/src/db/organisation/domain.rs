use std::fmt::Display;

use sqlx::{MySql, Transaction};

use crate::{
	models::db_mapping::Domain,
	query,
	query_as,
	utils::constants::AccountType,
};

pub async fn initialize_domain_pre(
	transaction: &mut Transaction<'_, MySql>,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing domain tables");
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS generic_domain (
			id BINARY(16),
			name VARCHAR(255) UNIQUE NOT NULL,
			type ENUM('personal','organisation'),
			PRIMARY KEY(id ,type)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS personal_domain (
			id BINARY(16) NOT NULL,
			domain_type ENUM('personal','organisation') NOT NULL,
			PRIMARY KEY(id),
			/* Change name of the constraint */
			CONSTRAINT p_fk FOREIGN KEY(id, domain_type) REFERENCES generic_domain(id, type),
			CONSTRAINT type_domain CHECK(domain_type='personal')
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS organisation_domain (
			id BINARY(16) NOT NULL,
			domain_type ENUM('personal','organisation') NOT NULL,
			is_verified BOOL NOT NULL DEFAULT FALSE,
			PRIMARY KEY(id),
			CONSTRAINT FOREIGN KEY(id, domain_type) REFERENCES generic_domain(id,type),
			CONSTRAINT CHECK(domain_type='organisation')
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	// wasn't sure about where to put this query, so for now i am adding it here
	// i plan to create a function similar to initialize_domain_post

	Ok(())
}

pub async fn initialize_domain_post(
	transaction: &mut Transaction<'_, MySql>,
) -> Result<(), sqlx::Error> {
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

pub async fn add_domain(
	connection: &mut Transaction<'_, MySql>,
	domain_id: &[u8],
	domain_name: &str,
	domain_type: AccountType,
) -> Result<(), sqlx::Error> {
	let domain_type = domain_type.to_string();

	query!(
		r#"
		INSERT INTO
			generic_domain
		VALUES
			(?, ?, ?);
		"#,
		domain_id,
		domain_name,
		domain_type
	)
	.execute(connection)
	.await?;

	Ok(())
}

pub async fn get_domains_for_organisation(
	connection: &mut Transaction<'_, MySql>,
	organisation_id: &[u8],
) -> Result<Vec<Domain>, sqlx::Error> {
	query_as!(
		Domain,
		r#"
		SELECT
			generic_domain.name,
			organisation_domain.is_verified as 'is_verified: bool'
		FROM
			generic_domain
		INNER JOIN
			organisation_domain
		ON
			organisation_domain.id = generic_domain.id
		INNER JOIN
			resource 
		ON
			generic_domain.id = resource.id
		WHERE
			resource.owner_id = ?;
		"#,
		organisation_id
	)
	.fetch_all(connection)
	.await
}

pub async fn get_all_unverified_domains(
	connection: &mut Transaction<'_, MySql>,
) -> Result<Vec<Domain>, sqlx::Error> {
	query_as!(
		Domain,
		r#"
		SELECT
			id,
			is_verified as `is_verified: bool`
		FROM
			organisation_domain
		WHERE
			is_verified = FALSE;
		"#
	)
	.fetch_all(connection)
	.await
}

pub async fn set_domain_as_verified(
	connection: &mut Transaction<'_, MySql>,
	domain_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			organisation_domain
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
	query_as!(
		Domain,
		r#"
		SELECT
			generic_domain.id,
			generic_domain.name,
			organisation_domain.is_verified as `is_verified: bool`
		FROM
			generic_domain
		INNER JOIN
			organisation_domain
		ON
			generic_domain.id = organisation_domain.id
		WHERE
			organisation_domain.is_verified = TRUE;
		"#
	)
	.fetch_all(connection)
	.await
}

pub async fn set_domain_as_unverified(
	connection: &mut Transaction<'_, MySql>,
	domain_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			organisation_domain
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

//work in progress
// TODO get the correct email based on organisation or personal
pub async fn get_notification_email_for_domain(
	connection: &mut Transaction<'_, MySql>,
	domain_id: &[u8],
) -> Result<Option<String>, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT
			user.*,
			generic_domain.name
		FROM
			personal_domain
		INNER JOIN
			generic_domain
		ON
			generic_domain.id = personal_domain.id
		INNER JOIN
			resource
		ON
			personal_domain.id = resource.id
		INNER JOIN
			organisation
		ON
			resource.owner_id = organisation.id
		INNER JOIN
			user
		ON
			organisation.super_admin_id = user.id
		WHERE
			generic_domain.id = ?;
		"#,
		domain_id
	)
	.fetch_all(connection)
	.await?;

	if rows.is_empty() {
		return Ok(None);
	}
	let row = rows.into_iter().next().unwrap();

	let backup_email = row.backup_email_local.unwrap() + "@" + &row.name;

	Ok(Some(backup_email))
}

pub async fn delete_domain_from_organisation(
	connection: &mut Transaction<'_, MySql>,
	domain_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			organisation_domain
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
			generic_domain.id,
			generic_domain.name,
			organisation_domain.is_verified as `is_verified: bool`
		FROM
			generic_domain
		INNER JOIN
			organisation_domain
		ON
			organisation_domain.id = generic_domain.id
		WHERE
			generic_domain.id = ?
		"#,
		domain_id
	)
	.fetch_all(connection)
	.await?;

	Ok(rows.into_iter().next())
}

pub async fn get_domain_by_name(
	connection: &mut Transaction<'_, MySql>,
	domain_name: &str,
) -> Result<Option<Domain>, sqlx::Error> {
	let rows = query_as!(
		Domain,
		r#"
		SELECT
			generic_domain.id,
			generic_domain.name,
			organisation_domain.is_verified as `is_verified: bool`
		FROM
			generic_domain
		INNER JOIN
			organisation_domain
		ON
			generic_domain.id = organisation_domain.id
		WHERE
			generic_domain.name = ?;
		"#,
		domain_name
	)
	.fetch_all(connection)
	.await?;

	Ok(rows.into_iter().next())
}
