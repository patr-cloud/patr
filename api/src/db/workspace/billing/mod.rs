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
		CREATE TYPE TRANSACTION_TYPE AS ENUM(
			'credits',
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
		CREATE TYPE PAYMENT_METHOD_TYPE AS ENUM(
			'credit_card',
			'debit_card'
		);
	"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS product_info(
			id UUID CONSTRAINT product_info_pk PRIMARY KEY,
			name TEXT NOT NULL,
			description TEXT
		);
	"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS plans(
			id UUID CONSTRAINT plans_pk PRIMARY KEY,
			name TEXT NOT NULL,
			description TEXT,
			plan_type PLAN_TYPE NOT NULL,
			product_info_id UUID NOT NULL,
			price DECIMAL(10, 2) NOT NULL,
			quantity INTEGER NOT NULL,
			workspace_id UUID NOT NULL
		);
	"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS billable_service(
			id UUID CONSTRAINT billable_service_pk PRIMARY KEY,
			plan_id UUId NOT NULL,
			workspace_id UUID NOT NULL,
			price DECIMAL(10, 2) NOT NULL,
			quantity INTEGER NOT NULL,
			product_info_id UUID NOT NULL,
			total_price DECIMAL(10, 2) NOT NULL
		);
	"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS transactions(
			id UUID CONSTRAINT transactions_pk PRIMARY KEY,
			billable_service_id UUID,
			product_info_id UUID NOT NULL,
			transaction_type TRANSACTION_TYPE NOT NULL,
			plan_id UUID,
			amount DECIMAL(10, 2) NOT NULL,
			quantity INTEGER NOT NULL,
			workspace_id UUID NOT NULL,
			date TIMESTAMPTZ NOT NULL,
			coupon_id UUID,
			debit bool NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS coupons(
			id UUID CONSTRAINT coupons_pk PRIMARY KEY,
			name TEXT NOT NULL,
			description TEXT,
			credits DECIMAL(10, 2) NOT NULL,
			valid_from TIMESTAMPTZ NOT NULL,
			valid_till TIMESTAMPTZ NOT NULL
		);
	"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS resource_limits(
			id UUID CONSTRAINT resource_limits_pk PRIMARY KEY,
			workspace_id UUID NOT NULL,
			max_resources INTEGER NOT NULL
		);
	"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS product_limits(
			id UUID CONSTRAINT resource_usage_pk PRIMARY KEY,
			product_info_id UUID NOT NULL,
			max_limit INTEGER NOT NULL,
			workspace_id UUID NOT NULL
		);
	"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS payment_method(
			id TEXT CONSTRAINT payment_method_pk PRIMARY KEY,
			method_type PAYMENT_METHOD_TYPE NOT NULL,
			workspace_id UUID NOT NULL
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
	log::info!("Finishing up deployment tables initialization");

	query!(
		r#"
		ALTER TABLE plans
		ADD CONSTRAINT plans_fk_workspace_id
		FOREIGN KEY(workspace_id) REFERENCES workspace(id);
	"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE plans
		ADD CONSTRAINT plans_fk_product_info_id
		FOREIGN KEY(product_info_id) REFERENCES product_info(id);
	"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE billable_service
		ADD CONSTRAINT billable_service_fk_workspace_id
		FOREIGN KEY(workspace_id) REFERENCES workspace(id);
	"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE billable_service
		ADD CONSTRAINT billable_service_fk_plan_id
		FOREIGN KEY(plan_id) REFERENCES plans(id);
	"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE transactions
		ADD CONSTRAINT transactions_fk_workspace_id
		FOREIGN KEY(workspace_id) REFERENCES workspace(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE transactions
		ADD CONSTRAINT transactions_fk_product_info_id
		FOREIGN KEY(product_info_id) REFERENCES product_info(id);
	"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE transactions
		ADD CONSTRAINT transactions_fk_plan_id
		FOREIGN KEY(plan_id) REFERENCES plans(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE transactions
		ADD CONSTRAINT transactions_fk_billable_service_id
		FOREIGN KEY(billable_service_id) REFERENCES billable_service(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE transactions
		ADD CONSTRAINT transactions_fk_coupon_id
		FOREIGN KEY(coupon_id) REFERENCES coupons(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE resource_limits
		ADD CONSTRAINT resource_limits_fk_workspace_id
		FOREIGN KEY(workspace_id) REFERENCES workspace(id)
		DEFERRABLE INITIALLY IMMEDIATE;
	"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE product_limits
		ADD CONSTRAINT resource_usage_fk_product_id
		FOREIGN KEY(product_info_id) REFERENCES product_info(id);
	"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE product_limits
		ADD CONSTRAINT resource_usage_fk_workspace_id
		FOREIGN KEY(workspace_id) REFERENCES workspace(id);
	"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE payment_method
		ADD CONSTRAINT payment_method_fk_workspace_id
		FOREIGN KEY(workspace_id) REFERENCES workspace(id);
	"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE MATERIALIZED VIEW credits
		AS
			SELECT SUM(
					CASE debit
					WHEN 'true' THEN 
						-amount
					ELSE 
						amount
					END
				) AS credits
			FROM transactions
			WHERE 
				billable_service_id IS NULL AND
				product_info_id IS NULL AND
				plan_id IS NULL
			GROUP BY workspace_id
		WITH NO DATA;
		/*This clause specifies whether or not the materialized view should be populated at creation time. 
		If not, the materialized view will be flagged as unscannable and cannot be queried until REFRESH MATERIALIZED VIEW is used. */
	"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
