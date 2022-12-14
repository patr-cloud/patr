use api_models::utils::Uuid;
use chrono::{DateTime, Utc};
use futures::TryStreamExt;

use crate::{query, query_as, Database};

#[derive(sqlx::Type)]
#[sqlx(type_name = "MANAGED_URL_TYPE", rename_all = "snake_case")]
pub enum ManagedUrlType {
	ProxyToDeployment,
	ProxyToStaticSite,
	ProxyUrl,
	Redirect,
}

pub struct ManagedUrl {
	pub id: Uuid,
	pub sub_domain: String,
	pub domain_id: Uuid,
	pub path: String,
	pub url_type: ManagedUrlType,
	pub deployment_id: Option<Uuid>,
	pub port: Option<i32>,
	pub static_site_id: Option<Uuid>,
	pub url: Option<String>,
	pub workspace_id: Uuid,
	pub is_configured: bool,
	pub permanent_redirect: Option<bool>,
	pub http_only: Option<bool>,
	pub cf_custom_hostname_id: Option<String>,
}

pub async fn initialize_managed_url_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing managed_url tables");
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

	// todo: add better constraint where
	// (sub_domain, domain_id, cf_custom_hostname_id)
	// should be same across all tables
	query!(
		r#"
		CREATE TABLE managed_url(
			id UUID CONSTRAINT managed_url_pk PRIMARY KEY,
			sub_domain TEXT NOT NULL
				CONSTRAINT managed_url_chk_sub_domain_valid CHECK(
					sub_domain ~ '^(([a-z0-9_]|[a-z0-9_][a-z0-9_\-]*[a-z0-9_])\.)*([a-z0-9_]|[a-z0-9_][a-z0-9_\-]*[a-z0-9_])$' OR
					sub_domain = '@'
				),
			domain_id UUID NOT NULL,
			path TEXT NOT NULL,
			url_type MANAGED_URL_TYPE NOT NULL,
			deployment_id UUID,
			port INTEGER CONSTRAINT managed_url_chk_port_u16 CHECK(
					port > 0 AND port <= 65535
				),
			static_site_id UUID,
			url TEXT,
			workspace_id UUID NOT NULL,
			is_configured BOOLEAN NOT NULL,
			deleted TIMESTAMPTZ,
			permanent_redirect BOOLEAN,
			http_only BOOLEAN,
			cf_custom_hostname_id TEXT,
			CONSTRAINT managed_url_chk_values_null_or_not_null CHECK(
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
			)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn initialize_managed_url_post(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Finishing up managed_url tables initialization");
	/* TODO remove some of these unnecessarry foreign key constraints after
	 * the permission checks are moved to the code */
	query!(
		r#"
		ALTER TABLE managed_url
			ADD CONSTRAINT managed_url_fk_domain_id
				FOREIGN KEY(domain_id) REFERENCES workspace_domain(id),
			ADD CONSTRAINT managed_url_fk_domain_id_workspace_id
				FOREIGN KEY(domain_id, workspace_id)
					REFERENCES resource(id, owner_id),
			ADD CONSTRAINT managed_url_fk_deployment_id_port
				FOREIGN KEY(deployment_id, port)
					REFERENCES deployment_exposed_port(deployment_id, port)
					DEFERRABLE INITIALLY IMMEDIATE,
			ADD CONSTRAINT managed_url_fk_deployment_id_workspace_id
				FOREIGN KEY(deployment_id, workspace_id)
					REFERENCES deployment(id, workspace_id),
			ADD CONSTRAINT managed_url_fk_static_site_id_workspace_id
				FOREIGN KEY(static_site_id, workspace_id)
					REFERENCES static_site(id, workspace_id),
			ADD CONSTRAINT managed_url_fk_id_workspace_id
				FOREIGN KEY(id, workspace_id)
					REFERENCES resource(id, owner_id);
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

pub async fn get_all_managed_urls_in_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<Vec<ManagedUrl>, sqlx::Error> {
	query_as!(
		ManagedUrl,
		r#"
		SELECT
			id as "id: _",
			sub_domain,
			domain_id as "domain_id: _",
			path,
			url_type as "url_type: _",
			deployment_id as "deployment_id: _",
			port,
			static_site_id as "static_site_id: _",
			url,
			workspace_id as "workspace_id: _",
			is_configured,
			permanent_redirect as "permanent_redirect: _",
			http_only as "http_only: _",
			cf_custom_hostname_id
		FROM
			managed_url
		WHERE
			workspace_id = $1 AND
			deleted IS NULL;
		"#,
		workspace_id as _
	)
	.fetch_all(connection)
	.await
}

pub async fn get_all_managed_urls_for_domain(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_id: &Uuid,
) -> Result<Vec<ManagedUrl>, sqlx::Error> {
	query_as!(
		ManagedUrl,
		r#"
		SELECT
			id as "id: _",
			sub_domain,
			domain_id as "domain_id: _",
			path,
			url_type as "url_type: _",
			deployment_id as "deployment_id: _",
			port,
			static_site_id as "static_site_id: _",
			url,
			workspace_id as "workspace_id: _",
			is_configured,
			permanent_redirect as "permanent_redirect: _",
			http_only as "http_only: _",
			cf_custom_hostname_id
		FROM
			managed_url
		WHERE
			domain_id = $1 AND
			deleted IS NULL;
		"#,
		domain_id as _
	)
	.fetch_all(connection)
	.await
}

pub async fn get_all_unconfigured_managed_urls(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Vec<ManagedUrl>, sqlx::Error> {
	query_as!(
		ManagedUrl,
		r#"
		SELECT
			id as "id: _",
			sub_domain,
			domain_id as "domain_id: _",
			path,
			url_type as "url_type: _",
			deployment_id as "deployment_id: _",
			port,
			static_site_id as "static_site_id: _",
			url,
			workspace_id as "workspace_id: _",
			is_configured,
			permanent_redirect as "permanent_redirect: _",
			http_only as "http_only: _",
			cf_custom_hostname_id
		FROM
			managed_url
		WHERE
			is_configured = FALSE AND
			deleted IS NULL;
		"#
	)
	.fetch_all(connection)
	.await
}

pub async fn get_all_configured_managed_urls(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Vec<ManagedUrl>, sqlx::Error> {
	query_as!(
		ManagedUrl,
		r#"
		SELECT
			id as "id: _",
			sub_domain,
			domain_id as "domain_id: _",
			path,
			url_type as "url_type: _",
			deployment_id as "deployment_id: _",
			port,
			static_site_id as "static_site_id: _",
			url,
			workspace_id as "workspace_id: _",
			is_configured,
			cf_custom_hostname_id
		FROM
			managed_url
		WHERE
			is_configured = TRUE AND
			deleted IS NULL;
		"#
	)
	.fetch_all(connection)
	.await
}

pub async fn create_new_managed_url_in_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	managed_url_id: &Uuid,
	sub_domain: &str,
	domain_id: &Uuid,
	path: &str,
	url_type: &ManagedUrlType,
	deployment_id: Option<&Uuid>,
	port: Option<u16>,
	static_site_id: Option<&Uuid>,
	url: Option<&str>,
	workspace_id: &Uuid,
	is_configured: bool,
	permanent_redirect: Option<bool>,
	http_only: Option<bool>,
	cf_custom_hostname_id: Option<String>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			managed_url(
				id,
				sub_domain,
				domain_id,
				path,
				url_type,
				deployment_id,
				port,
				static_site_id,
				url,
				workspace_id,
				is_configured,
				permanent_redirect,
				http_only,
				cf_custom_hostname_id
			)
		VALUES
			(
				$1,
				$2,
				$3,
				$4,
				$5,
				$6,
				$7,
				$8,
				$9,
				$10,
				$11,
				$12,
				$13,
				$14
			);
		"#,
		managed_url_id as _,
		sub_domain,
		domain_id as _,
		path,
		url_type as _,
		deployment_id as _,
		port.map(|port| port as i32),
		static_site_id as _,
		url,
		workspace_id as _,
		is_configured,
		permanent_redirect,
		http_only,
		cf_custom_hostname_id
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_managed_url_by_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	managed_url_id: &Uuid,
) -> Result<Option<ManagedUrl>, sqlx::Error> {
	query_as!(
		ManagedUrl,
		r#"
		SELECT
			id as "id: _",
			sub_domain,
			domain_id as "domain_id: _",
			path,
			url_type as "url_type: _",
			deployment_id as "deployment_id: _",
			port,
			static_site_id as "static_site_id: _",
			url,
			workspace_id as "workspace_id: _",
			is_configured,
			permanent_redirect as "permanent_redirect: _",
			http_only as "http_only: _",
			cf_custom_hostname_id
		FROM
			managed_url
		WHERE
			id = $1 AND
			deleted IS NULL;
		"#,
		managed_url_id as _,
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn update_managed_url(
	connection: &mut <Database as sqlx::Database>::Connection,
	managed_url_id: &Uuid,
	path: &str,
	url_type: &ManagedUrlType,
	deployment_id: Option<&Uuid>,
	port: Option<u16>,
	static_site_id: Option<&Uuid>,
	url: Option<&str>,
	permanent_redirect: Option<bool>,
	http_only: Option<bool>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			managed_url
		SET
			path = $2,
			url_type = $3,
			deployment_id = $4,
			port = $5,
			static_site_id = $6,
			url = $7,
			permanent_redirect = $8,
			http_only = $9
		WHERE
			id = $1;
		"#,
		managed_url_id as _,
		path,
		url_type as _,
		deployment_id as _,
		port.map(|port| port as i32),
		static_site_id as _,
		url,
		permanent_redirect,
		http_only,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_managed_url_configuration_status(
	connection: &mut <Database as sqlx::Database>::Connection,
	managed_url_id: &Uuid,
	is_configured: bool,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			managed_url
		SET
			is_configured = $2
		WHERE
			id = $1;
		"#,
		managed_url_id as _,
		is_configured
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn delete_managed_url(
	connection: &mut <Database as sqlx::Database>::Connection,
	managed_url_id: &Uuid,
	deletion_time: &DateTime<Utc>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			managed_url
		SET
			deleted = $2
		WHERE
			id = $1;
		"#,
		managed_url_id as _,
		deletion_time
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_all_managed_urls_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
	workspace_id: &Uuid,
) -> Result<Vec<ManagedUrl>, sqlx::Error> {
	query_as!(
		ManagedUrl,
		r#"
		WITH RECURSIVE managed_url_list AS (
			SELECT
				managed_url.*
			FROM
				managed_url
			INNER JOIN
				domain
			ON
				managed_url.domain_id = domain.id
			WHERE
				managed_url.deployment_id = $1 AND
				managed_url.url_type = 'proxy_to_deployment'
			UNION ALL
			SELECT
				managed_url.*
			FROM
				managed_url
			INNER JOIN
				domain
			ON
				managed_url.domain_id = domain.id
			JOIN
				managed_url_list
			ON
				managed_url.url LIKE CONCAT(
					managed_url.sub_domain,
					'.',
					domain.name,
					'.',
					domain.tld,
					CASE managed_url.path
						WHEN '/' THEN
							'%'
						ELSE
							CONCAT(managed_url.path, '%')
					END
				)
			WHERE
				managed_url.url_type = 'redirect' OR
				managed_url.url_type = 'proxy_url'
		) SELECT
			id as "id!: _",
			sub_domain as "sub_domain!: _",
			domain_id as "domain_id!: _",
			path as "path!: _",
			url_type as "url_type!: _",
			deployment_id as "deployment_id: _",
			port,
			static_site_id as "static_site_id: _",
			url,
			workspace_id as "workspace_id!: _",
			is_configured as "is_configured!: _",
			permanent_redirect as "permanent_redirect!: _",
			http_only as "http_only!: _",
			cf_custom_hostname_id
		FROM
			managed_url_list
		WHERE
			managed_url_list.workspace_id = $2 AND
			managed_url_list.deleted IS NULL;
		"#,
		deployment_id as _,
		workspace_id as _
	)
	.fetch_all(connection)
	.await
}

pub async fn get_all_managed_urls_for_static_site(
	connection: &mut <Database as sqlx::Database>::Connection,
	static_site_id: &Uuid,
	workspace_id: &Uuid,
) -> Result<Vec<ManagedUrl>, sqlx::Error> {
	query_as!(
		ManagedUrl,
		r#"
		WITH RECURSIVE managed_url_list AS (
			SELECT
				managed_url.*
			FROM
				managed_url
			INNER JOIN
				domain
			ON
				managed_url.domain_id = domain.id
			WHERE
				managed_url.static_site_id = $1 AND
				managed_url.url_type = 'proxy_to_static_site'
			UNION ALL
			SELECT
				managed_url.*
			FROM
				managed_url
			INNER JOIN
				domain
			ON
				managed_url.domain_id = domain.id
			JOIN
				managed_url_list
			ON
				managed_url.url LIKE CONCAT(
					managed_url.sub_domain,
					'.',
					domain.name,
					'.',
					domain.tld,
					CASE managed_url.path
						WHEN '/' THEN
							'%'
						ELSE
							CONCAT(managed_url.path, '%')
					END
				)
			WHERE
				managed_url.url_type = 'redirect' OR
				managed_url.url_type = 'proxy_url'
		) SELECT
			id as "id!: _",
			sub_domain as "sub_domain!: _",
			domain_id as "domain_id!: _",
			path as "path!: _",
			url_type as "url_type!: _",
			deployment_id as "deployment_id: _",
			port,
			static_site_id as "static_site_id: _",
			url,
			workspace_id as "workspace_id!: _",
			is_configured as "is_configured!: _",
			permanent_redirect as "permanent_redirect!: _",
			http_only as "http_only!: _",
			cf_custom_hostname_id
		FROM
			managed_url_list
		WHERE
			managed_url_list.workspace_id = $2 AND
			managed_url_list.deleted IS NULL;
		"#,
		static_site_id as _,
		workspace_id as _
	)
	.fetch_all(connection)
	.await
}

pub async fn get_active_managed_url_count_for_domain(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_id: &Uuid,
) -> Result<i64, sqlx::Error> {
	query!(
		r#"
		SELECT
			COUNT(*) as "count"
		FROM
			managed_url
		WHERE
			domain_id = $1 AND
			deleted IS NULL;
		"#,
		domain_id as _
	)
	.fetch_one(&mut *connection)
	.await
	.map(|row| row.count.unwrap_or(0))
}

pub async fn get_all_managed_urls_for_host(
	connection: &mut <Database as sqlx::Database>::Connection,
	sub_domain: &str,
	domain_id: &Uuid,
) -> Result<Vec<ManagedUrl>, sqlx::Error> {
	query_as!(
		ManagedUrl,
		r#"
		SELECT
			id as "id: _",
			sub_domain,
			domain_id as "domain_id: _",
			path,
			url_type as "url_type: _",
			deployment_id as "deployment_id: _",
			port,
			static_site_id as "static_site_id: _",
			url,
			workspace_id as "workspace_id: _",
			is_configured,
			cf_custom_hostname_id
		FROM
			managed_url
		WHERE
			deleted IS NULL AND
			sub_domain = $1 AND
			domain_id = $2;
		"#,
		sub_domain,
		domain_id as _
	)
	.fetch_all(connection)
	.await
}

pub async fn get_all_deployment_managed_urls_for_host_in_region(
	connection: &mut <Database as sqlx::Database>::Connection,
	sub_domain: &str,
	domain_id: &Uuid,
	workspace_id: &Uuid,
	region_id: &Uuid,
) -> Result<Vec<(String, Uuid, i32)>, sqlx::Error> {
	query!(
		r#"
		SELECT
			path,
			deployment_id as "deployment_id!: Uuid",
			port as "port!"
		FROM
			managed_url
		JOIN
			deployment ON deployment.id = managed_url.deployment_id
		WHERE
			managed_url.deleted IS NULL AND
			deployment_id IS NOT NULL AND
			port IS NOT NULL AND
			sub_domain = $1 AND
			domain_id = $2 AND
			deployment.region = $3 AND
			managed_url.workspace_id = $4;
		"#,
		sub_domain,
		domain_id as _,
		region_id as _,
		workspace_id as _,
	)
	.fetch(connection)
	.map_ok(|record| (record.path, record.deployment_id, record.port))
	.try_collect()
	.await
}
