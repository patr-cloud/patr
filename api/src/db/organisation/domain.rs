use sqlx::{MySql, Transaction};

use crate::{
	models::db_mapping::{GenericDomain, OrganisationDomain, PersonalDomain},
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
			id BINARY(16) PRIMARY KEY,
			name VARCHAR(255) UNIQUE NOT NULL,
			type ENUM('personal', 'organisation') NOT NULL,
			KEY(id, type)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS personal_domain (
			id BINARY(16) PRIMARY KEY,
			domain_type ENUM('personal', 'organisation') NOT NULL,
			/* Change name of the constraint */
			FOREIGN KEY(id, domain_type) REFERENCES generic_domain(id, type),
			CONSTRAINT type_domain CHECK(domain_type = 'personal')
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS organisation_domain (
			id BINARY(16) PRIMARY KEY,
			domain_type ENUM('personal', 'organisation') NOT NULL,
			is_verified BOOL NOT NULL DEFAULT FALSE,
			FOREIGN KEY(id, domain_type) REFERENCES generic_domain(id, type),
			CONSTRAINT CHECK(domain_type = 'organisation')
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	Ok(())
}

pub async fn initialize_domain_post(
	transaction: &mut Transaction<'_, MySql>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TABLE generic_domain
		ADD CONSTRAINT
		FOREIGN KEY(id) REFERENCES resource(id);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	Ok(())
}

pub async fn add_to_generic_domain(
	connection: &mut Transaction<'_, MySql>,
	domain_id: &[u8],
	domain_name: &str,
	domain_type: AccountType,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			generic_domain
		VALUES
			(?, ?, ?);
		"#,
		domain_id,
		domain_name,
		domain_type.to_string()
	)
	.execute(connection)
	.await?;

	Ok(())
}

pub async fn add_to_organisation_domain(
	connection: &mut Transaction<'_, MySql>,
	domain_id: &[u8],
	is_verified: bool,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			organisation_domain
		VALUES
			(?, 'organisation', ?);
		"#,
		domain_id,
		is_verified
	)
	.execute(connection)
	.await?;

	Ok(())
}

pub async fn add_to_personal_domain(
	connection: &mut Transaction<'_, MySql>,
	domain_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			personal_domain
		VALUES
			(?, 'personal');
		"#,
		domain_id,
	)
	.execute(connection)
	.await?;

	Ok(())
}

pub async fn get_personal_domain_by_name(
	connection: &mut Transaction<'_, MySql>,
	domain_name: &str,
) -> Result<Option<PersonalDomain>, sqlx::Error> {
	let rows = query_as!(
		PersonalDomain,
		r#"
		SELECT
			personal_domain.id,
			generic_domain.name
		FROM
			generic_domain
		INNER JOIN
			personal_domain
		ON
			personal_domain.id = generic_domain.id
		WHERE
			generic_domain.name = ?;
		"#,
		domain_name
	)
	.fetch_all(connection)
	.await?;

	Ok(rows.into_iter().next())
}

pub async fn get_domains_for_organisation(
	connection: &mut Transaction<'_, MySql>,
	organisation_id: &[u8],
) -> Result<Vec<OrganisationDomain>, sqlx::Error> {
	query_as!(
		OrganisationDomain,
		r#"
		SELECT
			generic_domain.id,
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

pub async fn get_all_unverified_organisation_domains(
	connection: &mut Transaction<'_, MySql>,
) -> Result<Vec<OrganisationDomain>, sqlx::Error> {
	query_as!(
		OrganisationDomain,
		r#"
		SELECT
			organisation_domain.id,
			organisation_domain.is_verified as `is_verified: bool`,
			generic_domain.name
		FROM
			organisation_domain
		INNER JOIN
			generic_domain
		ON
			generic_domain.id = organisation_domain.id
		WHERE
			is_verified = FALSE;
		"#
	)
	.fetch_all(connection)
	.await
}

pub async fn set_organisation_domain_as_verified(
	connection: &mut Transaction<'_, MySql>,
	domain_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			organisation_domain
		SET
			is_verified = true
		WHERE
			id = ?;
		"#,
		domain_id
	)
	.execute(connection)
	.await?;

	Ok(())
}

pub async fn get_all_verified_organisation_domains(
	connection: &mut Transaction<'_, MySql>,
) -> Result<Vec<OrganisationDomain>, sqlx::Error> {
	query_as!(
		OrganisationDomain,
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

pub async fn set_organisation_domain_as_unverified(
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

// TODO make email notification permission-based
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
			organisation_domain
		INNER JOIN
			generic_domain
		ON
			generic_domain.id = organisation_domain.id
		INNER JOIN
			resource
		ON
			organisation_domain.id = resource.id
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

pub async fn get_organisation_domain_by_id(
	connection: &mut Transaction<'_, MySql>,
	domain_id: &[u8],
) -> Result<Option<OrganisationDomain>, sqlx::Error> {
	let rows = query_as!(
		OrganisationDomain,
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
) -> Result<Option<GenericDomain>, sqlx::Error> {
	let rows = query_as!(
		GenericDomain,
		r#"
		SELECT
			*
		FROM
			generic_domain
		WHERE
			name = ?;
		"#,
		domain_name
	)
	.fetch_all(connection)
	.await?;

	Ok(rows.into_iter().next())
}
