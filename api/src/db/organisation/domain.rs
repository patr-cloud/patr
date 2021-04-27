use sqlx::{MySql, Transaction};

use crate::{models::db_mapping::Domain, query, query_as};

pub async fn initialize_domain_pre(
	transaction: &mut Transaction<'_, MySql>,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing domain tables");
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS domain (
			id BINARY(16),
			name VARCHAR(255) UNIQUE NOT NULL,
			type ENUM('personal','organisation'),
			is_verified BOOL NOT NULL DEFAULT FALSE,
			PRIMARY KEY(id, name ,type)
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
			domain_name VARCHAR(255) NOT NULL,
			PRIMARY KEY(id, name),
			CONSTRAINT p_fk FOREIGN KEY(id,domain_name, domain_type) REFERENCES domain(id,name, type),
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
			domain_name VARCHAR(255) NOT NULL,
			PRIMARY KEY(id, name),
			CONSTRAINT o_fk FOREIGN KEY(id,domain_name, domain_type) REFERENCES domain(id,type),
			CONSTRAINT type_domain CHECK(domain_type='organisation')
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	// wasn't sure about where to put this query, so for now i am adding it here
	// i plan to create a function simialr to initialize_domain_post
	query!(
		r#"
		ALTER TABLE personal_email
		ADD CONSTRAINT 
		dom_p_fk FOREIGN KEY(email_domain,domain_id) REFERENCES personal_domain(domain_name,id);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		ALTER TABLE organisation_email
		ADD CONSTRAINT 
		dom_o_fk FOREIGN KEY(email_domain,domain_id) REFERENCES organisation_domain(domain_name,id);
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
	domain_type: &str,
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
			domain.id,
			domain.name,
			is_verified as `is_verified: bool`
		FROM
			domain
		INNER JOIN
			resource
		ON
			domain.id = resource.id
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
			name,
			is_verified as `is_verified: bool`
		FROM
			domain
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
	query_as!(
		Domain,
		r#"
		SELECT
			id,
			name,
			is_verified as `is_verified: bool`
		FROM
			domain
		WHERE
			is_verified = TRUE;
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

// TODO get the correct email based on organisation or personal
// pub async fn get_notification_email_for_domain(
// 	connection: &mut Transaction<'_, MySql>,
// 	domain_id: &[u8],
// ) -> Result<Option<String>, sqlx::Error> {
// 	let rows = query!(
// 		r#"
// 		SELECT
// 			user.id,
// 			user.username,
// 			user.password,
// 			CONCAT(user.backup_email_local,'@',user.backup_email_domain)
// 			AS
// 			'backup_email',
// 			u
// 			user.first_name,
// 			user.last_name,
// 			user.dob,
// 			user.bio,
// 			user.
// 		FROM
// 			domain
// 		INNER JOIN
// 			resource
// 		ON
// 			domain.id = resource.id
// 		INNER JOIN
// 			organisation
// 		ON
// 			resource.owner_id = organisation.id
// 		INNER JOIN
// 			user
// 		ON
// 			organisation.super_admin_id = user.id
// 		WHERE
// 			domain.id = ?;
// 		"#,
// 		domain_id
// 	)
// 	.fetch_all(connection)
// 	.await?;

// 	if rows.is_empty() {
// 		return Ok(None);
// 	}
// 	let row = rows.into_iter().next().unwrap();

// 	Ok(Some(row.backup_email))
// }

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
			is_verified as `is_verified: bool`
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

pub async fn get_domain_by_name(
	connection: &mut Transaction<'_, MySql>,
	domain_name: &str,
) -> Result<Option<Domain>, sqlx::Error> {
	let rows = query_as!(
		Domain,
		r#"
		SELECT
			id,
			name,
			is_verified as `is_verified: bool`
		FROM
			domain
		WHERE
			name = ?;
		"#,
		domain_name
	)
	.fetch_all(connection)
	.await?;

	Ok(rows.into_iter().next())
}
