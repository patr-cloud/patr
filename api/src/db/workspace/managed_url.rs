use crate::prelude::*;

/// Initializes the managed URL tables
#[instrument(skip(connection))]
pub async fn initialize_managed_url_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up managed_url tables");
	query!(
		r#"
		CREATE TYPE MANAGED_URL_TYPE AS ENUM(
			'proxy_to_deployment',
			'proxy_to_static_site',
			'proxy_url',
			'redirect'
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE managed_url(
			id UUID NOT NULL,
			sub_domain TEXT NOT NULL,
			domain_id UUID NOT NULL,
			path TEXT NOT NULL,
			url_type MANAGED_URL_TYPE NOT NULL,
			deployment_id UUID,
			port INTEGER,
			static_site_id UUID,
			url TEXT,
			workspace_id UUID NOT NULL,
			is_configured BOOLEAN NOT NULL,
			deleted TIMESTAMPTZ,
			permanent_redirect BOOLEAN,
			http_only BOOLEAN,
			cloudflare_custom_hostname_id TEXT NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes the managed URL indices
#[instrument(skip(connection))]
pub async fn initialize_managed_url_indices(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up managed_url tables indices");
	query!(
		r#"
		ALTER TABLE managed_url
		ADD CONSTRAINT managed_url_pk
		PRIMARY KEY(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE UNIQUE INDEX
			managed_url_uq_sub_domain_domain_id_path
		ON
			managed_url(sub_domain, domain_id, path)
		WHERE
			deleted IS NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes the managed URL constraints
#[instrument(skip(connection))]
pub async fn initialize_managed_url_constraints(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up managed_url tables constraints");
	query!(
		r#"
		ALTER TABLE managed_url
			ADD CONSTRAINT managed_url_chk_sub_domain_valid CHECK(
				sub_domain ~ '^(([a-z0-9_]|[a-z0-9_][a-z0-9_\-]*[a-z0-9_])\.)*([a-z0-9_]|[a-z0-9_][a-z0-9_\-]*[a-z0-9_])$' OR
				sub_domain = '@'
			),
			ADD CONSTRAINT managed_url_chk_port_u16 CHECK(
				port > 0 AND port <= 65535
			),
			ADD CONSTRAINT managed_url_chk_values_null_or_not_null CHECK(
				(
					url_type = 'proxy_to_deployment' AND
					deployment_id IS NOT NULL AND
					port IS NOT NULL AND
					static_site_id IS NULL AND
					url IS NULL AND
					permanent_redirect IS NULL AND
					http_only IS NULL
				) OR (
					url_type = 'proxy_to_static_site' AND
					deployment_id IS NULL AND
					port IS NULL AND
					static_site_id IS NOT NULL AND
					url IS NULL AND
					permanent_redirect IS NULL AND
					http_only IS NULL
				) OR (
					url_type = 'proxy_url' AND
					deployment_id IS NULL AND
					port IS NULL AND
					static_site_id IS NULL AND
					url IS NOT NULL AND
					permanent_redirect IS NULL AND
					http_only IS NOT NULL
				) OR (
					url_type = 'redirect' AND
					deployment_id IS NULL AND
					port IS NULL AND
					static_site_id IS NULL AND
					url IS NOT NULL AND
					permanent_redirect IS NOT NULL AND
					http_only IS NOT NULL
				)
			),
			ADD CONSTRAINT managed_url_fk_domain_id
				FOREIGN KEY(domain_id) REFERENCES workspace_domain(id),
			ADD CONSTRAINT managed_url_fk_id_workspace_id_deleted
				FOREIGN KEY(id, workspace_id, deleted)
					REFERENCES resource(id, owner_id, deleted)
					DEFERRABLE INITIALLY IMMEDIATE,
			ADD CONSTRAINT managed_url_fk_deployment_id_port
				FOREIGN KEY(deployment_id, port)
					REFERENCES deployment_exposed_port(deployment_id, port)
						DEFERRABLE INITIALLY IMMEDIATE,
			ADD CONSTRAINT managed_url_fk_deployment_id_workspace_id
				FOREIGN KEY(deployment_id, workspace_id) REFERENCES deployment(id, workspace_id),
			ADD CONSTRAINT managed_url_fk_static_site_id_workspace_id
				FOREIGN KEY(static_site_id, workspace_id) REFERENCES static_site(id, workspace_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
