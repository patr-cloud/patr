use crate::{migrate_query as query, utils::settings::Settings, Database};

pub async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), sqlx::Error> {
	// Remove old tables.
	query!(
		r#"
		DROP TABLE data_center_locations;
		"#
	)
	.execute(&mut *connection)
	.await?;

	// TODO Move deployed_domain to managed_urls:
	// Find the domains in deployed_domain.
	// Create those domains as user-controlled domains, but unverified.
	// Add them all as proxy_deployment or proxy_static_site to managed_urls.

	query!(
		r#"
		DROP TABLE deployed_domain CASCADE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DROP TABLE deployment_request_logs;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DROP TYPE DEPLOYMENT_REQUEST_METHOD;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DROP TYPE DEPLOYMENT_REQUEST_PROTOCOL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE DEPLOYMENT_CLOUD_PROVIDER AS ENUM(
			'digitalocean'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE deployment_region(
			id UUID CONSTRAINT deployment_region_pk PRIMARY KEY,
			name TEXT NOT NULL,
			provider DEPLOYMENT_CLOUD_PROVIDER,
			location GEOMETRY,
			parent_region_id UUID
				CONSTRAINT deployment_region_fk_parent_region_id
					REFERENCES deployment_region(id),
			CONSTRAINT
				deployment_region_chk_provider_location_parent_region_is_valid
				CHECK(
					(
						location IS NULL AND
						provider IS NULL
					) OR
					(
						provider IS NOT NULL AND
						location IS NOT NULL AND
						parent_region_id IS NOT NULL
					)
				)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// TODO migrate all DO apps to k8s before doing this
	// TODO migrate all regions to UUIDs before doing this
	// TODO migrate all machine types to UUIDs before doing this
	query!(
		r#"
		ALTER TABLE deployment
			DROP COLUMN deployed_image,
			DROP COLUMN digitalocean_app_id,
			DROP COLUMN region,
			DROP COLUMN domain_name CASCADE,
			DROP COLUMN horizontal_scale,
			DROP COLUMN machine_type;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment_static_sites
		DROP COLUMN domain_name CASCADE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DROP TYPE DEPLOYMENT_MACHINE_TYPE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE deployment_machine_type(
			id UUID CONSTRAINT deployment_machint_type_pk PRIMARY KEY,
			cpu_count SMALLINT NOT NULL,
			memory_count INTEGER NOT NULL /* Multiples of 0.25 GB */
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment
			ADD COLUMN region UUID NOT NULL CONSTRAINT deployment_fk_region
				REFERENCES deployment_region(id),
			ADD COLUMN min_horizontal_scale SMALLINT NOT NULL
				CONSTRAINT deployment_chk_min_horizontal_scale_u8 CHECK(
					min_horizontal_scale >= 0 AND min_horizontal_scale <= 256
				)
				DEFAULT 1,
			ADD COLUMN max_horizontal_scale SMALLINT NOT NULL
				CONSTRAINT deployment_chk_max_horizontal_scale_u8 CHECK(
					max_horizontal_scale >= 0 AND max_horizontal_scale <= 256
				)
				DEFAULT 1,
			ADD COLUMN machine_type UUID NOT NULL
				CONSTRAINT deployment_fk_machine_type
					REFERENCES deployment_machine_type(id),
			ADD COLUMN deploy_on_push BOOLEAN NOT NULL DEFAULT TRUE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE EXPOSED_PORT_TYPE AS ENUM(
			'http'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE deployment_exposed_port(
			deployment_id UUID
				CONSTRAINT deployment_exposed_port_fk_deployment_id
					REFERENCES deployment(id),
			port SMALLINT NOT NULL CONSTRAINT
				deployment_exposed_port_chk_port_u16 CHECK(
					port > 0 AND port <= 65535
				),
			port_type EXPOSED_PORT_TYPE NOT NULL,
			CONSTRAINT deployment_exposed_port_pk
				PRIMARY KEY(deployment_id, port)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
