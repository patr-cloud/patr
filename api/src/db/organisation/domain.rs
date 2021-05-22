use sqlx::Transaction;
use uuid::Uuid;

use crate::{
	models::db_mapping::{Domain, OrganisationDomain, PersonalDomain},
	query,
	query_as,
	utils::constants::ResourceOwnerType,
	Database,
};

pub async fn initialize_domain_pre(
	transaction: &mut Transaction<'_, Database>,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing domain tables");

	query!(
		r#"
		CREATE TABLE domain(
			id BYTEA CONSTRAINT domain_pk PRIMARY KEY,
			name VARCHAR(255) NOT NULL CONSTRAINT domain_uq_name UNIQUE,
			type RESOURCE_OWNER_TYPE NOT NULL,
			CONSTRAINT domain_uq_name_type UNIQUE(id, type)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE organisation_domain (
			id BYTEA CONSTRAINT organisation_domain_pk PRIMARY KEY,
			domain_type RESOURCE_OWNER_TYPE NOT NULL
				CONSTRAINT organisation_domain_chk_dmn_typ
					CHECK(domain_type = 'organisation'),
			is_verified BOOLEAN NOT NULL DEFAULT FALSE,
			CONSTRAINT organisation_domain_fk_id_domain_type
				FOREIGN KEY(id, domain_type) REFERENCES domain(id, type)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE INDEX
			organisation_domain_idx_is_verified
		ON
			organisation_domain
		(is_verified);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE personal_domain (
			id BYTEA
				CONSTRAINT personal_domain_pk PRIMARY KEY,
			domain_type RESOURCE_OWNER_TYPE NOT NULL
				CONSTRAINT personal_domain_chk_dmn_typ
					CHECK(domain_type = 'personal'),
			CONSTRAINT personal_domain_fk_id_domain_type
				FOREIGN KEY(id, domain_type) REFERENCES domain(id, type)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	Ok(())
}

pub async fn initialize_domain_post(
	transaction: &mut Transaction<'_, Database>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TABLE organisation_domain
		ADD CONSTRAINT organisation_domain_fk_id
		FOREIGN KEY(id) REFERENCES resource(id);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	Ok(())
}

pub async fn generate_new_domain_id(
	connection: &mut Transaction<'_, Database>,
) -> Result<Uuid, sqlx::Error> {
	let mut uuid = Uuid::new_v4();

	// If it exists in the resource table, it can't be used
	// because organisation domains are a resource
	// If it exists in the domain table, it can't be used
	// since personal domains are a type of domains
	let mut exists = {
		query!(
			r#"
			SELECT
				*
			FROM
				resource
			WHERE
				id = $1;
			"#,
			uuid.as_bytes().as_ref()
		)
		.fetch_all(&mut *connection)
		.await?
		.into_iter()
		.next()
		.is_some()
	} || {
		query!(
			r#"
			SELECT
				id,
				name,
				type as "type: ResourceOwnerType"
			FROM
				domain
			WHERE
				id = $1;
			"#,
			uuid.as_bytes().as_ref()
		)
		.fetch_all(&mut *connection)
		.await?
		.into_iter()
		.next()
		.is_some()
	};

	while exists {
		uuid = Uuid::new_v4();
		exists = {
			query!(
				r#"
				SELECT
					*
				FROM
					resource
				WHERE
					id = $1;
				"#,
				uuid.as_bytes().as_ref()
			)
			.fetch_all(&mut *connection)
			.await?
			.into_iter()
			.next()
			.is_some()
		} || {
			query!(
				r#"
				SELECT
					id,
					name,
					type as "type: ResourceOwnerType"
				FROM
					domain
				WHERE
					id = $1;
				"#,
				uuid.as_bytes().as_ref()
			)
			.fetch_all(&mut *connection)
			.await?
			.into_iter()
			.next()
			.is_some()
		}
	}

	Ok(uuid)
}

pub async fn create_generic_domain(
	connection: &mut Transaction<'_, Database>,
	domain_id: &[u8],
	domain_name: &str,
	domain_type: &ResourceOwnerType,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			domain
		VALUES
			($1, $2, $3);
		"#,
		domain_id,
		domain_name,
		domain_type as _
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn add_to_organisation_domain(
	connection: &mut Transaction<'_, Database>,
	domain_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			organisation_domain
		VALUES
			($1, 'organisation', FALSE);
		"#,
		domain_id
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn add_to_personal_domain(
	connection: &mut Transaction<'_, Database>,
	domain_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			personal_domain
		VALUES
			($1, 'personal');
		"#,
		domain_id
	)
	.execute(connection)
	.await
	.map(|_| ())
}

pub async fn get_domains_for_organisation(
	connection: &mut Transaction<'_, Database>,
	organisation_id: &[u8],
) -> Result<Vec<OrganisationDomain>, sqlx::Error> {
	query_as!(
		OrganisationDomain,
		r#"
		SELECT
			domain.name,
			organisation_domain.id,
			organisation_domain.domain_type as "domain_type: _",
			organisation_domain.is_verified
		FROM
			domain
		INNER JOIN
			organisation_domain
		ON
			organisation_domain.id = domain.id
		INNER JOIN
			resource
		ON
			domain.id = resource.id
		WHERE
			resource.owner_id = $1;
		"#,
		organisation_id
	)
	.fetch_all(connection)
	.await
}

pub async fn get_all_unverified_domains(
	connection: &mut Transaction<'_, Database>,
) -> Result<Vec<OrganisationDomain>, sqlx::Error> {
	query_as!(
		OrganisationDomain,
		r#"
		SELECT
			domain.name as "name!",
			organisation_domain.id as "id!",
			organisation_domain.domain_type as "domain_type!: _",
			organisation_domain.is_verified as "is_verified!"
		FROM
			organisation_domain
		INNER JOIN
			domain
		ON
			domain.id = organisation_domain.id
		WHERE
			is_verified = FALSE;
		"#
	)
	.fetch_all(connection)
	.await
}

pub async fn set_domain_as_verified(
	connection: &mut Transaction<'_, Database>,
	domain_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			organisation_domain
		SET
			is_verified = TRUE
		WHERE
			id = $1;
		"#,
		domain_id
	)
	.execute(connection)
	.await
	.map(|_| ())
}

pub async fn get_all_verified_domains(
	connection: &mut Transaction<'_, Database>,
) -> Result<Vec<OrganisationDomain>, sqlx::Error> {
	query_as!(
		OrganisationDomain,
		r#"
		SELECT
			domain.name as "name!",
			organisation_domain.id as "id!",
			organisation_domain.domain_type as "domain_type!: _",
			organisation_domain.is_verified as "is_verified!"
		FROM
			organisation_domain
		INNER JOIN
			domain
		ON
			domain.id = organisation_domain.id
		WHERE
			is_verified = TRUE;
		"#
	)
	.fetch_all(connection)
	.await
}

pub async fn set_domain_as_unverified(
	connection: &mut Transaction<'_, Database>,
	domain_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			organisation_domain
		SET
			is_verified = FALSE
		WHERE
			id = $1;
		"#,
		domain_id
	)
	.execute(connection)
	.await
	.map(|_| ())
}

// TODO get the correct email based on permission
pub async fn get_notification_email_for_domain(
	connection: &mut Transaction<'_, Database>,
	domain_id: &[u8],
) -> Result<Option<String>, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT
			"user".*,
			domain.name
		FROM
			organisation_domain
		INNER JOIN
			domain
		ON
			domain.id = organisation_domain.id
		INNER JOIN
			resource
		ON
			organisation_domain.id = resource.id
		INNER JOIN
			organisation
		ON
			resource.owner_id = organisation.id
		INNER JOIN
			"user"
		ON
			organisation.super_admin_id = "user".id
		WHERE
			organisation_domain.id = $1;
		"#,
		domain_id
	)
	.fetch_all(connection)
	.await?;

	if rows.is_empty() {
		return Ok(None);
	}
	let row = rows.into_iter().next().unwrap();

	Ok(Some(format!(
		"{}@{}",
		row.backup_email_local.unwrap(),
		row.name
	)))
}

#[allow(dead_code)]
pub async fn delete_personal_domain(
	connection: &mut Transaction<'_, Database>,
	domain_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			personal_domain
		WHERE
			id = $1;
		"#,
		domain_id
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn delete_domain_from_organisation(
	connection: &mut Transaction<'_, Database>,
	domain_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			organisation_domain
		WHERE
			id = $1;
		"#,
		domain_id
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn delete_generic_domain(
	connection: &mut Transaction<'_, Database>,
	domain_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			domain
		WHERE
			id = $1;
		"#,
		domain_id
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn get_organisation_domain_by_id(
	connection: &mut Transaction<'_, Database>,
	domain_id: &[u8],
) -> Result<Option<OrganisationDomain>, sqlx::Error> {
	let rows = query_as!(
		OrganisationDomain,
		r#"
		SELECT
			domain.name,
			organisation_domain.id,
			organisation_domain.domain_type as "domain_type: _",
			organisation_domain.is_verified
		FROM
			organisation_domain
		INNER JOIN
			domain
		ON
			domain.id = organisation_domain.id
		WHERE
			domain.id = $1;
		"#,
		domain_id
	)
	.fetch_all(connection)
	.await?;

	Ok(rows.into_iter().next())
}

pub async fn get_personal_domain_by_id(
	connection: &mut Transaction<'_, Database>,
	domain_id: &[u8],
) -> Result<Option<PersonalDomain>, sqlx::Error> {
	let rows = query_as!(
		PersonalDomain,
		r#"
		SELECT
			domain.name,
			personal_domain.id,
			personal_domain.domain_type as "domain_type: _"
		FROM
			personal_domain
		INNER JOIN
			domain
		ON
			domain.id = personal_domain.id
		WHERE
			domain.id = $1;
		"#,
		domain_id
	)
	.fetch_all(connection)
	.await?;

	Ok(rows.into_iter().next())
}

pub async fn get_domain_by_name(
	connection: &mut Transaction<'_, Database>,
	domain_name: &str,
) -> Result<Option<Domain>, sqlx::Error> {
	let rows = query_as!(
		Domain,
		r#"
		SELECT
			id,
			name,
			type as "type: _"
		FROM
			domain
		WHERE
			name = $1;
		"#,
		domain_name
	)
	.fetch_all(connection)
	.await?;

	Ok(rows.into_iter().next())
}
