use crate::prelude::*;

/// Initializes the domain tables
#[instrument(skip(connection))]
pub async fn initialize_domain_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up domain tables");
	query!(
		r#"
		CREATE TABLE domain_tld(
			tld TEXT NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE DOMAIN_NAMESERVER_TYPE AS ENUM(
			'internal',
			'external'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE workspace_domain(
			id UUID NOT NULL,
			name TEXT NOT NULL,
			tld TEXT NOT NULL,
			workspace_id UUID NOT NULL,
			nameserver_type DOMAIN_NAMESERVER_TYPE NOT NULL,
			is_verified BOOLEAN NOT NULL,
			deleted TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE patr_controlled_domain(
			domain_id UUID NOT NULL,
			zone_identifier TEXT NOT NULL,
			nameserver_type DOMAIN_NAMESERVER_TYPE NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE user_controlled_domain(
			domain_id UUID NOT NULL,
			nameserver_type DOMAIN_NAMESERVER_TYPE NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE DNS_RECORD_TYPE AS ENUM(
			'A',
			'MX',
			'TXT',
			'AAAA',
			'CNAME'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE patr_domain_dns_record(
			id UUID NOT NULL,
			record_identifier TEXT NOT NULL,
			domain_id UUID NOT NULL,
			name TEXT NOT NULL,
			type DNS_RECORD_TYPE NOT NULL,
			value TEXT NOT NULL,
			priority INTEGER,
			ttl BIGINT NOT NULL,
			proxied BOOLEAN
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes the domain indices
#[instrument(skip(connection))]
pub async fn initialize_domain_indices(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up domain tables indices");

	query!(
		r#"
		ALTER TABLE domain_tld
		ADD CONSTRAINT domain_tld_pk
		PRIMARY KEY(tld);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace_domain
			ADD CONSTRAINT workspace_domain_pk PRIMARY KEY(id),
			ADD CONSTRAINT workspace_domain_uq_id_nameserver_type
				UNIQUE(id, nameserver_type);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE patr_controlled_domain
		ADD CONSTRAINT patr_controlled_domain_pk
		PRIMARY KEY(domain_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_controlled_domain
		ADD CONSTRAINT User_controlled_domain_pk
		PRIMARY KEY(domain_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE patr_domain_dns_record
			ADD CONSTRAINT patr_domain_dns_record_pk PRIMARY KEY(id),
			ADD CONSTRAINT patr_domain_dns_record_uq_domain_id_name_type_value_priority
				UNIQUE(domain_id, name, type, value, priority);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE UNIQUE INDEX
			workspace_domain_uq_name_tld
		ON
			workspace_domain(name, tld)
		WHERE
			deleted IS NULL;
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

	Ok(())
}

/// Initializes the domain constraints
#[instrument(skip(connection))]
pub async fn initialize_domain_constraints(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up domain tables constraints");
	query!(
		r#"
		ALTER TABLE domain_tld
			ADD CONSTRAINT domain_tld_chk_is_length_valid CHECK(
				LENGTH(tld) >= 2 AND LENGTH(tld) <= 63
			),
			ADD CONSTRAINT domain_tld_chk_is_tld_valid CHECK(
				tld ~ '^(([a-z0-9])|([a-z0-9][a-z0-9\-\.]*[a-z0-9]))$'
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace_domain
			ADD CONSTRAINT workspace_domain_chk_name_is_valid CHECK(
				name ~ '^(([a-z0-9])|([a-z0-9][a-z0-9-]*[a-z0-9]))$'
			),
			ADD CONSTRAINT workspace_domain_chk_max_domain_name_length CHECK(
				(LENGTH(name) + LENGTH(tld)) < 255
			),
			ADD CONSTRAINT workspace_domain_fk_tld FOREIGN KEY(tld) REFERENCES domain_tld(tld),
			ADD CONSTRAINT workspace_domain_fk_workspace_id
				FOREIGN KEY(workspace_id) REFERENCES workspace(id),
			ADD CONSTRAINT workspace_domain_fk_id_deleted
				FOREIGN KEY(id, workspace_id, deleted)
					REFERENCES resource(id, owner_id, deleted)
					DEFERRABLE INITIALLY IMMEDIATE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE patr_controlled_domain
			ADD CONSTRAINT patr_controlled_domain_chk_nameserver_type CHECK(
				nameserver_type = 'internal'
			),
			ADD	CONSTRAINT patr_controlled_domain_fk_domain_id_nameserver_type
				FOREIGN KEY(domain_id, nameserver_type)
					REFERENCES workspace_domain(id, nameserver_type);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_controlled_domain
			ADD CONSTRAINT user_controlled_domain_chk_nameserver_type CHECK(
				nameserver_type = 'external'
			),
			ADD CONSTRAINT user_controlled_domain_fk_domain_id_nameserver_type
				FOREIGN KEY(domain_id, nameserver_type)	
					REFERENCES workspace_domain(id, nameserver_type);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE patr_domain_dns_record
			ADD CONSTRAINT patr_domain_dns_record_fk_id
				FOREIGN KEY(id) REFERENCES resource(id),
			ADD CONSTRAINT patr_domain_dns_record_chk_name_is_valid CHECK(
				name ~ '^((\*)|((\*\.)?(([a-z0-9_]|[a-z0-9_][a-z0-9_\-]*[a-z0-9_])\.)*([a-z0-9_]|[a-z0-9_][a-z0-9_\-]*[a-z0-9_])))$' OR
				name = '@'
			),
			ADD CONSTRAINT patr_domain_dns_record_fk_domain_id
				FOREIGN KEY(domain_id) REFERENCES patr_controlled_domain(domain_id),
			ADD CONSTRAINT patr_domain_dns_record_chk_values_valid CHECK(
				(
					type = 'MX' AND priority IS NOT NULL
				) OR (
					type != 'MX' AND priority IS NULL
				)
			),
			ADD CONSTRAINT patr_domain_dns_record_chk_proxied_is_valid CHECK(
				(
					(type = 'A' OR type = 'AAAA' OR type = 'CNAME') AND
					proxied IS NOT NULL
				) OR
				(
					(type = 'MX' OR type = 'TXT') AND
					proxied IS NULL
				)
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
