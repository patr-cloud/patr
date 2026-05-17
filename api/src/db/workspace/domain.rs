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
		CREATE TABLE workspace_domain(
			id UUID NOT NULL,
			name TEXT NOT NULL,
			tld TEXT NOT NULL,
			workspace_id UUID NOT NULL,
			is_verified BOOLEAN NOT NULL,
			last_verified TIMESTAMPTZ,
			deleted TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE managed_url_custom_hostname(
			sub_domain TEXT NOT NULL,
			domain_id UUID NOT NULL,
			cloudflare_custom_hostname_id TEXT NOT NULL,
			is_active BOOLEAN NOT NULL DEFAULT FALSE,
			last_verified TIMESTAMPTZ
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
		ADD CONSTRAINT workspace_domain_pk PRIMARY KEY(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE managed_url_custom_hostname
		ADD CONSTRAINT managed_url_custom_hostname_pk
		PRIMARY KEY(sub_domain, domain_id);
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
		ALTER TABLE managed_url_custom_hostname
			ADD CONSTRAINT managed_url_custom_hostname_fk_domain_id
				FOREIGN KEY(domain_id)
					REFERENCES workspace_domain(id),
			ADD CONSTRAINT managed_url_custom_hostname_chk_sub_domain_valid CHECK(
				sub_domain = '@' OR
				sub_domain ~ '^(([a-z0-9_]|[a-z0-9_][a-z0-9_\-]*[a-z0-9_])\.)*([a-z0-9_]|[a-z0-9_][a-z0-9_\-]*[a-z0-9_])$'
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
