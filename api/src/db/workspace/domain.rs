use api_models::utils::Uuid;

use crate::{
	models::db_mapping::{
		DnsRecord,
		Domain,
		DomainControlStatus,
		PatrControlledDomain,
		PersonalDomain,
		WorkspaceDomain,
	},
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
			id UUID CONSTRAINT domain_pk PRIMARY KEY,
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
		CREATE TYPE DOMAIN_CONTROL_STATUS AS ENUM (
			'patr',
			'user'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE workspace_domain (
			id UUID CONSTRAINT workspace_domain_pk PRIMARY KEY,
			domain_type RESOURCE_OWNER_TYPE NOT NULL
				CONSTRAINT workspace_domain_chk_dmn_typ
					CHECK(domain_type = 'business'),
			is_verified BOOLEAN NOT NULL DEFAULT FALSE,
			control_status DOMAIN_CONTROL_STATUS NOT NULL DEFAULT 'user'::DOMAIN_CONTROL_STATUS,
			CONSTRAINT workspace_domain_fk_id_domain_type
				FOREIGN KEY(id, domain_type) REFERENCES domain(id, type)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE personal_domain (
			id UUID CONSTRAINT personal_domain_pk PRIMARY KEY,
			domain_type RESOURCE_OWNER_TYPE NOT NULL
				CONSTRAINT personal_domain_chk_domain_type
					CHECK(domain_type = 'personal'),
			CONSTRAINT personal_domain_fk_id_domain_type
				FOREIGN KEY(id, domain_type) REFERENCES domain(id, type)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// todo: create composite key for this table
	query!(
		r#"
		CREATE TABLE patr_controlled_domain (
			domain_id UUID NOT NULL REFERENCES domain(id),
			zone_identifier TEXT NOT NULL,
			control_status DOMAIN_CONTROL_STATUS NOT NULL DEFAULT 'patr',
			CONSTRAINT patr_controlled_domain_chk_control_status CHECK(
				control_status = 'patr'
			)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE user_controlled_domain (
			domain_id UUID NOT NULL REFERENCES domain(id),
			control_status DOMAIN_CONTROL_STATUS NOT NULL DEFAULT 'user',
			CONSTRAINT user_controlled_domain_chk_control_status CHECK(
				control_status = 'user'
			)
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

	// todo: check if MX record exists for domain, then other records should be
	// null and vice versa
	// todo: remove path and rename sub_domain to name
	query!(
		r#"
		CREATE TABLE dns_record (
			id UUID CONSTRAINT dns_record_pk PRIMARY KEY,
			domain_id UUID NOT NULL,
			name TEXT NOT NULL
				CONSTRAINT dns_record_chk_name_is_lower_case CHECK(
					name = LOWER(name)
				)
				CONSTRAINT dns_record_chk_name_is_trimmed CHECK(
					name = TRIM(name)
				),
			a_record TEXT [] NOT NULL DEFAULT '{}',
			aaaa_record TEXT [] NOT NULL DEFAULT '{}',
			text_record TEXT [] NOT NULL DEFAULT '{}',
			cname_record TEXT NOT NULL DEFAULT '',
			mx_record TEXT [] NOT NULL DEFAULT '{}',
			ttl INTEGER NOT NULL,
			priority INTEGER NOT NULL DEFAULT 0,
			proxied BOOLEAN NOT NULL DEFAULT TRUE,
			CONSTRAINT dns_record_uq_domain_id_sub_domain_path UNIQUE (domain_id, name),
			CONSTRAINT dns_record_fk_domain_id FOREIGN KEY (domain_id) REFERENCES domain(id)
		);
		"#
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
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
	.await
	.map(|_| ())
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
				uuid as _
			)
			.fetch_optional(&mut *connection)
			.await?
			.is_some()
		} || {
			query!(
				r#"
				SELECT
					id
				FROM
					domain
				WHERE
					id = $1;
				"#,
				uuid as _
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
	domain_id: &Uuid,
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
		domain_id as _,
		domain_name,
		domain_type as _
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn add_to_workspace_domain(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_id: &Uuid,
	control_status: &DomainControlStatus,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			workspace_domain
		VALUES
			($1, 'business', FALSE, $2);
		"#,
		domain_id as _,
		control_status as _
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn add_to_personal_domain(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			personal_domain
		VALUES
			($1, 'personal');
		"#,
		domain_id as _
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_domains_for_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<Vec<WorkspaceDomain>, sqlx::Error> {
	query_as!(
		WorkspaceDomain,
		r#"
		SELECT
			domain.name,
			workspace_domain.id as "id: _",
			workspace_domain.domain_type as "domain_type: _",
			workspace_domain.is_verified,
			workspace_domain.control_status as "control_status: _"
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
		workspace_id as _
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
			workspace_domain.id as "id!: _",
			workspace_domain.domain_type as "domain_type!: _",
			workspace_domain.is_verified as "is_verified!",
			workspace_domain.control_status as "control_status!: _"
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
	domain_id: &Uuid,
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
		domain_id as _
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
			workspace_domain.id as "id!: _",
			workspace_domain.domain_type as "domain_type!: _",
			workspace_domain.is_verified as "is_verified!",
			workspace_domain.control_status as "control_status!: _"
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
	domain_id: &Uuid,
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
		domain_id as _
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

// TODO get the correct email based on permission
pub async fn get_notification_email_for_domain(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_id: &Uuid,
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
		domain_id as _
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
	domain_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			personal_domain
		WHERE
			id = $1;
		"#,
		domain_id as _
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn delete_domain_from_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			workspace_domain
		WHERE
			id = $1;
		"#,
		domain_id as _
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn delete_generic_domain(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			domain
		WHERE
			id = $1;
		"#,
		domain_id as _
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_workspace_domain_by_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_id: &Uuid,
) -> Result<Option<WorkspaceDomain>, sqlx::Error> {
	query_as!(
		WorkspaceDomain,
		r#"
		SELECT
			domain.name,
			workspace_domain.id as "id: _",
			workspace_domain.domain_type as "domain_type: _",
			workspace_domain.is_verified,
			workspace_domain.control_status as "control_status: _"
		FROM
			workspace_domain
		INNER JOIN
			domain
		ON
			domain.id = workspace_domain.id
		WHERE
			domain.id = $1;
		"#,
		domain_id as _
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_personal_domain_by_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_id: &Uuid,
) -> Result<Option<PersonalDomain>, sqlx::Error> {
	query_as!(
		PersonalDomain,
		r#"
		SELECT
			domain.name,
			personal_domain.id as "id: _",
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
		domain_id as _
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
			id as "id: _",
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

pub async fn add_patr_controlled_domain(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_id: &Uuid,
	zone_identifier: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			patr_controlled_domain
		VALUES
			($1, $2, 'false');
		"#,
		domain_id as _,
		zone_identifier,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_dns_record_by_domain_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_id: &Uuid,
) -> Result<Vec<DnsRecord>, sqlx::Error> {
	query_as!(
		DnsRecord,
		r#"
		SELECT
			id as "id: _",
			domain_id as "domain_id: _",
			name::TEXT as "name!: _",
			a_record,
			aaaa_record,
			text_record,
			cname_record,
			mx_record,
			ttl,
			priority,
			proxied
		FROM
			dns_record
		WHERE
			domain_id = $1;
		"#,
		domain_id as _
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn get_patr_controlled_domain_by_domain_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_id: &Uuid,
) -> Result<Option<PatrControlledDomain>, sqlx::Error> {
	query_as!(
		PatrControlledDomain,
		r#"
		SELECT
			domain_id as "domain_id: _",
			zone_identifier,
			control_status as "control_status: _"
		FROM
			patr_controlled_domain
		WHERE
			domain_id = $1;
		"#,
		domain_id as _
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_dns_record_by_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	dns_id: &Uuid,
) -> Result<Option<DnsRecord>, sqlx::Error> {
	query_as!(
		DnsRecord,
		r#"
		SELECT
			id as "id: _",
			domain_id as "domain_id: _",
			name::TEXT as "name!: _",
			a_record,
			aaaa_record,
			text_record,
			cname_record,
			mx_record,
			ttl,
			priority,
			proxied
		FROM
			dns_record
		WHERE
			id = $1;
		"#,
		dns_id as _
	)
	.fetch_optional(&mut *connection)
	.await
}

// ON CONFLICT reference: https://www.postgresqltutorial.com/postgresql-upsert/
pub async fn add_patr_dns_a_record(
	connection: &mut <Database as sqlx::Database>::Connection,
	id: &Uuid,
	domain_id: &Uuid,
	name: &str,
	content: &[String],
	ttl: i32,
	proxied: bool,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
			INSERT INTO
				dns_record
			VALUES
				($1, $2, $3, $4, default, default, default, default, $5, default, $6)
			ON CONFLICT
				(domain_id, name)
			DO UPDATE SET
				a_record = $4 || EXCLUDED.a_record;
		"#,
		id as _,
		domain_id as _,
		name,
		content,
		ttl,
		proxied,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn add_patr_dns_aaaa_record(
	connection: &mut <Database as sqlx::Database>::Connection,
	id: &Uuid,
	domain_id: &Uuid,
	name: &str,
	content: &[String],
	ttl: i32,
	proxied: bool,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
			INSERT INTO
				dns_record
			VALUES
				($1, $2, $3, default, $4, default, default, default, $5, default, $6)
			ON CONFLICT
				(domain_id, name)
			DO UPDATE SET
				aaaa_record = $4 || EXCLUDED.aaaa_record;
		"#,
		id as _,
		domain_id as _,
		name,
		content,
		ttl,
		proxied,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn add_patr_dns_mx_record(
	connection: &mut <Database as sqlx::Database>::Connection,
	id: &Uuid,
	domain_id: &Uuid,
	name: &str,
	content: &[String],
	ttl: i32,
	priority: i32,
	proxied: bool,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
			INSERT INTO
				dns_record
			VALUES
				($1, $2, $3, default, default, $4, default, default, $5, $6, $7)
			ON CONFLICT
				(domain_id, name)
			DO UPDATE SET
				mx_record = $4 || EXCLUDED.mx_record;
		"#,
		id as _,
		domain_id as _,
		name,
		content,
		ttl,
		priority,
		proxied,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn add_patr_dns_cname_record(
	connection: &mut <Database as sqlx::Database>::Connection,
	id: &Uuid,
	domain_id: &Uuid,
	name: &str,
	content: &str,
	ttl: i32,
	proxied: bool,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
			INSERT INTO
				dns_record
			VALUES
				($1, $2, $3, default, default, default, $4, default, $5, default, $6)
			ON CONFLICT
				(domain_id, name)
			DO UPDATE SET
				cname_record = $4;
		"#,
		id as _,
		domain_id as _,
		name,
		content,
		ttl,
		proxied,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn add_patr_dns_txt_record(
	connection: &mut <Database as sqlx::Database>::Connection,
	id: &Uuid,
	domain_id: &Uuid,
	name: &str,
	content: &[String],
	ttl: i32,
	proxied: bool,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
			INSERT INTO
				dns_record
			VALUES
				($1, $2, $3, default, default, $4, default, default, $5, default, $6)
			ON CONFLICT
				(domain_id, name)
			DO UPDATE SET
				text_record = $4 || EXCLUDED.text_record;
		"#,
		id as _,
		domain_id as _,
		name,
		content,
		ttl,
		proxied,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn delete_patr_controlled_dns_record(
	connection: &mut <Database as sqlx::Database>::Connection,
	dns_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
			DELETE FROM
				dns_record
			WHERE
				id = $1;
		"#,
		dns_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_workspace_domain_status(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_id: &Uuid,
	is_verified: bool,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
			UPDATE
				workspace_domain
			SET
				is_verified = $2
			WHERE
				id = $1;
		"#,
		domain_id as _,
		is_verified,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}
