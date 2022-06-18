use api_macros::query;

use crate::Database;

pub async fn initialize_billing_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing billing tables");

	query!(
		r#"
        CREATE TYPE DEPLOYMENT_ACTION AS ENUM(
            'create',
            'delete',
            'update',
            'start'
        );
        "#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
        CREATE TYPE STATIC_SITE_PLAN AS ENUM(
            'free',
            '25',
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
            '25',
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
			'unlimited'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
        CREATE TABLE IF NOT EXISTS deployment_payment_history(
            id UUID CONSTRAINT deployment_payment_history_pk PRIMARY KEY,
            workspace_id UUID NOT NULL,
            deployment_id UUID NOT NULL,
            machine_type UUID NOT NULL,
            num_instance INTEGER NOT NULL,
            time TIMESTAMPTZ NOT NULL,
            action DEPLOYMENT_ACTION NOT NULL
        );
        "#,
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
        CREATE TABLE IF NOT EXISTS static_sites_payment_history(
            id UUID CONSTRAINT static_sites_payment_history_pk PRIMARY KEY,
            workspace_id UUID NOT NULL,
            static_site_plan STATIC_SITE_PLAN NOT NULL,
            time TIMESTAMPTZ NOT NULL
        );
        "#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
        CREATE TABLE IF NOT EXISTS managed_database_payment_history(
            id UUID CONSTRAINT managed_database_payment_history_pk PRIMARY KEY,
            workspace_id UUID NOT NULL,
            database_id UUID NOT NULL,
            db_plan UUID NOT NULL,
            start_time TIMESTAMPTZ NOT NULL,
            deletion_time TIMESTAMPTZ NULL
        );
        "#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
        CREATE TABLE IF NOT EXISTS managed_url_payment_history(
            id UUID CONSTRAINT managed_url_payment_history_pk PRIMARY KEY,
            workspace_id UUID NOT NULL,
            url_count INTEGER NOT NULL
        );
        "#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
        CREATE TABLE IF NOT EXISTS secrets_payment_history(
            id UUID CONSTRAINT secrets_payment_history_pk PRIMARY KEY,
            workspace_id UUID NOT NULL,
            secret_count INTEGER NOT NULL
        );
        "#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
        CREATE TABLE IF NOT EXISTS docker_repo_payment_history(
            id UUID CONSTRAINT docker_repo_payment_history_pk PRIMARY KEY,
            workspace_id UUID NOT NULL,
            storage BIGINT NOT NULL
        );
        "#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
        CREATE TABLE IF NOT EXISTS domain_payment_history(
            id UUID CONSTRAINT domain_payment_history_pk PRIMARY KEY,
            workspace_id UUID NOT NULL,
            domain_plan DOMAIN_PLAN NOT NULL,
            time TIMESTAMPTZ NOT NULL
        );
        "#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn initialize_billing_post(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Finishing up billing tables initialization");

	query!(
		r#"
        ALTER TABLE deployment_payment_history 
        ADD CONSTRAINT deployment_payment_history_workspace_id_fk 
        FOREIGN KEY (workspace_id) REFERENCES workspace(id);
        "#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
        ALTER TABLE static_sites_payment_history
        ADD CONSTRAINT static_sites_payment_history_workspace_id_fk
        FOREIGN KEY (workspace_id) REFERENCES workspace(id);
        "#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
        ALTER TABLE managed_database_payment_history
        ADD CONSTRAINT managed_database_payment_history_workspace_id_fk
        FOREIGN KEY (workspace_id) REFERENCES workspace(id);
        "#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
        ALTER TABLE managed_url_payment_history
        ADD CONSTRAINT managed_url_payment_history_workspace_id_fk
        FOREIGN KEY (workspace_id) REFERENCES workspace(id);
        "#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
        ALTER TABLE secrets_payment_history
        ADD CONSTRAINT secrets_payment_history_workspace_id_fk
        FOREIGN KEY (workspace_id) REFERENCES workspace(id);
        "#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
        ALTER TABLE docker_repo_payment_history
        ADD CONSTRAINT docker_repo_payment_history_workspace_id_fk
        FOREIGN KEY (workspace_id) REFERENCES workspace(id);
        "#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
        ALTER TABLE domain_payment_history
        ADD CONSTRAINT domain_payment_history_workspace_id_fk
        FOREIGN KEY (workspace_id) REFERENCES workspace(id);
        "#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
