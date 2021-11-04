use uuid::Uuid;

use crate::{
	models::db_mapping::{Domain, PersonalDomain, WorkspaceDomain},
	query,
	query_as,
	utils::constants::ResourceOwnerType,
	Database,
};

pub async fn initialize_domain_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing domain tables");

	query!(
		r#"
		CREATE TABLE domain(
			id BYTEA CONSTRAINT domain_pk PRIMARY KEY,
			name VARCHAR(255) NOT NULL 
				CONSTRAINT domain_uq_name UNIQUE
				CONSTRAINT domain_chk_name_is_lower_case 
					CHECK(
						name = LOWER(name)
					),
			type RESOURCE_OWNER_TYPE NOT NULL,
			CONSTRAINT domain_uq_name_type UNIQUE(id, type)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE workspace_domain (
			id BYTEA CONSTRAINT workspace_domain_pk PRIMARY KEY,
			domain_type RESOURCE_OWNER_TYPE NOT NULL
				CONSTRAINT workspace_domain_chk_dmn_typ
					CHECK(domain_type = 'business'),
			is_verified BOOLEAN NOT NULL DEFAULT FALSE,
			CONSTRAINT workspace_domain_fk_id_domain_type
				FOREIGN KEY(id, domain_type) REFERENCES domain(id, type)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			workspace_domain_idx_is_verified
		ON
			workspace_domain
		(is_verified);
		"#
	)
	.execute(&mut *connection)
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
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn initialize_domain_post(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Finishing up domain tables initialization");
	query!(
		r#"
		ALTER TABLE workspace_domain
		ADD CONSTRAINT workspace_domain_fk_id
		FOREIGN KEY(id) REFERENCES resource(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn generate_new_domain_id(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Uuid, sqlx::Error> {
	loop {
		let uuid = Uuid::new_v4();

		// If it exists in the resource table, it can't be used
		// because workspace domains are a resource
		// If it exists in the domain table, it can't be used
		// since personal domains are a type of domains
		let exists = {
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
			.fetch_optional(&mut *connection)
			.await?
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
			.fetch_optional(&mut *connection)
			.await?
			.is_some()
		};

		if !exists {
			break Ok(uuid);
		}
	}
}

pub async fn create_generic_domain(
	connection: &mut <Database as sqlx::Database>::Connection,
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

pub async fn add_to_workspace_domain(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			workspace_domain
		VALUES
			($1, 'business', FALSE);
		"#,
		domain_id
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn add_to_personal_domain(
	connection: &mut <Database as sqlx::Database>::Connection,
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
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_domains_for_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &[u8],
) -> Result<Vec<WorkspaceDomain>, sqlx::Error> {
	query_as!(
		WorkspaceDomain,
		r#"
		SELECT
			domain.name,
			workspace_domain.id,
			workspace_domain.domain_type as "domain_type: _",
			workspace_domain.is_verified
		FROM
			domain
		INNER JOIN
			workspace_domain
		ON
			workspace_domain.id = domain.id
		INNER JOIN
			resource
		ON
			domain.id = resource.id
		WHERE
			resource.owner_id = $1;
		"#,
		workspace_id
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn get_all_unverified_domains(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Vec<WorkspaceDomain>, sqlx::Error> {
	query_as!(
		WorkspaceDomain,
		r#"
		SELECT
			domain.name as "name!",
			workspace_domain.id as "id!",
			workspace_domain.domain_type as "domain_type!: _",
			workspace_domain.is_verified as "is_verified!"
		FROM
			workspace_domain
		INNER JOIN
			domain
		ON
			domain.id = workspace_domain.id
		WHERE
			is_verified = FALSE;
		"#
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn set_domain_as_verified(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			workspace_domain
		SET
			is_verified = TRUE
		WHERE
			id = $1;
		"#,
		domain_id
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_all_verified_domains(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Vec<WorkspaceDomain>, sqlx::Error> {
	query_as!(
		WorkspaceDomain,
		r#"
		SELECT
			domain.name as "name!",
			workspace_domain.id as "id!",
			workspace_domain.domain_type as "domain_type!: _",
			workspace_domain.is_verified as "is_verified!"
		FROM
			workspace_domain
		INNER JOIN
			domain
		ON
			domain.id = workspace_domain.id
		WHERE
			is_verified = TRUE;
		"#
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn set_domain_as_unverified(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			workspace_domain
		SET
			is_verified = FALSE
		WHERE
			id = $1;
		"#,
		domain_id
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

// TODO get the correct email based on permission
pub async fn get_notification_email_for_domain(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_id: &[u8],
) -> Result<Option<String>, sqlx::Error> {
	let email = query!(
		r#"
		SELECT
			"user".*,
			domain.name
		FROM
			workspace_domain
		INNER JOIN
			domain
		ON
			domain.id = workspace_domain.id
		INNER JOIN
			resource
		ON
			workspace_domain.id = resource.id
		INNER JOIN
			workspace
		ON
			resource.owner_id = workspace.id
		INNER JOIN
			"user"
		ON
			workspace.super_admin_id = "user".id
		WHERE
			workspace_domain.id = $1;
		"#,
		domain_id
	)
	.fetch_optional(&mut *connection)
	.await?
	.map(|row| {
		if let Some(email_local) = row.backup_email_local {
			Ok(format!("{}@{}", email_local, row.name))
		} else {
			Err(sqlx::Error::RowNotFound)
		}
	})
	.transpose()?;

	Ok(email)
}

pub async fn delete_personal_domain(
	connection: &mut <Database as sqlx::Database>::Connection,
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

pub async fn delete_domain_from_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			workspace_domain
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
	connection: &mut <Database as sqlx::Database>::Connection,
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

pub async fn get_workspace_domain_by_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_id: &[u8],
) -> Result<Option<WorkspaceDomain>, sqlx::Error> {
	query_as!(
		WorkspaceDomain,
		r#"
		SELECT
			domain.name,
			workspace_domain.id,
			workspace_domain.domain_type as "domain_type: _",
			workspace_domain.is_verified
		FROM
			workspace_domain
		INNER JOIN
			domain
		ON
			domain.id = workspace_domain.id
		WHERE
			domain.id = $1;
		"#,
		domain_id
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_personal_domain_by_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_id: &[u8],
) -> Result<Option<PersonalDomain>, sqlx::Error> {
	query_as!(
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
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_domain_by_name(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_name: &str,
) -> Result<Option<Domain>, sqlx::Error> {
	query_as!(
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
	.fetch_optional(&mut *connection)
	.await
}
