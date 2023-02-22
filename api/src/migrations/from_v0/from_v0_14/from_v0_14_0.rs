use crate::{
	migrate_query as query,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	new_pricing_tables(connection, config).await?;
	Ok(())
}

pub(super) async fn new_pricing_tables(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		CREATE TYPE DEPLOYMENT_PLAN AS ENUM(
			'free',
			'pro'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE STATIC_SITE_PLAN AS ENUM(
			'free',
			'pro',
			'unlimited'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE DATABASE_PLAN AS ENUM(
			'free',
			'unlimited'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE SECRETS_PLAN AS ENUM(
			'free',
			'pro',
			'unlimited'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE DOMAIN_PLAN AS ENUM(
			'free',
			'pro',
			'unlimited'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE MANAGED_URL_PLAN AS ENUM(
			'free',
			'pro',
			'unlimited'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE DOCKER_REPO_PLAN AS ENUM(
			'free',
			'pro',
			'unlimited'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE deployment_pricing(
			id UUID CONSTRAINT deployment_pricing_pk PRIMARY KEY,
			plan DEPLOYMENT_PLAN NOT NULL,
			machine_type_id UUID NOT NULL CONSTRAINT
				deployment_pricing_fk_machine_type
					REFERENCES deployment_machine_type(id),
			number_of_deployments INT NOT NULL,
			price_in_cents BIGINT NOT NULL,
			min_replica INT NOT NULL,
			max_replica INT NOT NULL,
			start_time TIMESTAMPTZ NOT NULL,
			start_time TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE static_site_pricing(
			id UUID CONSTRAINT static_site_pricing_pk PRIMARY KEY,
			plan STATIC_SITE_PLAN NOT NULL,
			number_of_sites INT NOT NULL,
			price_in_cents BIGINT NOT NULL,
			start_time TIMESTAMPTZ NOT NULL,
			start_time TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE domain_pricing(
			id UUID CONSTRAINT domain_pricing_pk PRIMARY KEY,
			plan DOMAIN_PLAN NOT NULL,
			number_of_domains INT NOT NULL,
			price_in_cents BIGINT NOT NULL,
			start_time TIMESTAMPTZ NOT NULL,
			start_time TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE managed_url_pricing(
			id UUID CONSTRAINT managed_url_pricing_pk PRIMARY KEY,
			plan MANAGED_URL_PLAN NOT NULL,
			number_of_urls INT NOT NULL,
			price_in_cents BIGINT NOT NULL,
			start_time TIMESTAMPTZ NOT NULL,
			start_time TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE managed_database_pricing(
			id UUID CONSTRAINT managed_database_pricing_pk PRIMARY KEY,
			plan DATABASE_PLAN NOT NULL,
			managed_database_machine_plan TEXT NOT NULL,
			number_of_databases INT NOT NULL,
			price_in_cents BIGINT NOT NULL,
			start_time TIMESTAMPTZ NOT NULL,
			start_time TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE secret_pricing(
			id UUID CONSTRAINT secrets_pricing_pk PRIMARY KEY,
			plan SECRETS_PLAN NOT NULL,
			number_of_secrets INT NOT NULL,
			price_in_cents BIGINT NOT NULL,
			start_time TIMESTAMPTZ NOT NULL,
			start_time TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE docker_repository_pricing(
			id UUID NOT NULL PRIMARY KEY,
			plan DOCKER_REPO_PLAN NOT NULL,
			storage_in_byte BIGINT NOT NULL,
			price_in_cents BIGINT NOT NULL,
			start_time TIMESTAMPTZ NOT NULL,
			stop_time TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE workspace_pricing(
			workspace_id UUID NOT NULL,
			pricing_id UUID NOT NULL,
			resource_type_id UUID NOT NULL,
			CONSTRAINT workspace_pricing_uq_pricing_id_resource_type_id
				UNIQUE(workspace_id, pricing_id, resource_type_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
