use api_macros::query;

use crate::Database;

pub async fn initialize_billing_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing billing tables");
	query!(
		r#"
		CREATE TYPE PLAN_TYPE AS ENUM(
            'one_time',
            'fixed_monthly',
            'dynamic_monthly'
            /* 'fixed_annually', */
            /* 'dynamic_annually', */
            /* 'fixed_resources', */

		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
        CREATE TABLE IF NOT EXISTS billable_service(
            id UUID
                CONSTRAINT billable_service_pk_id PRIMARY KEY,
            name CITEXT NOT NULL,
            description TEXT,
            plan_type PLAN_TYPE NOT NULL,
            
        )
    "#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn initialize_billing_post(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	Ok(())
}
