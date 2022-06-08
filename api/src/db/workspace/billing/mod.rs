use api_macros::{query, query_as};
use api_models::utils::{DateTime, Uuid};
use chrono::Utc;

use crate::Database;

#[derive(sqlx::Type)]
#[sqlx(type_name = "PLAN_TYPE", rename_all = "snake_case")]
pub enum PlanType {
	OneTime,
	FixedMonthly,
	DynamicMonthly,
}

#[derive(sqlx::Type)]
#[sqlx(type_name = "TRANSACTION_TYPE", rename_all = "snake_case")]
pub enum TransactionType {
	Onetime,
	FixedMonthly,
	DynamicMonthly,
}

#[derive(sqlx::Type)]
#[sqlx(type_name = "PAYMENT_METHOD_TYPE", rename_all = "snake_case")]
pub enum PaymentMethodType {
	CreditCard,
	DebitCard,
}

pub struct ProductInfo {
	pub id: Uuid,
	pub name: String,
	pub description: Option<String>,
}

pub struct Plans {
	pub id: Uuid,
	pub name: String,
	pub description: Option<String>,
	pub plan_type: PlanType,
	pub product_info_id: Uuid,
	pub price: f64,
	pub quantity: Option<i32>,
	pub workspace_id: String,
}

pub struct BillableServiceUsage {
	pub id: Uuid,
	pub plan_id: Uuid,
	pub workspace_id: Uuid,
	pub price: f64,
	pub quantity: i32,
	pub product_info_id: Uuid,
	pub total_price: f64,
	pub resource_id: Uuid,
	pub date: DateTime<Utc>,
	pub active: bool,
}

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
			quantity INTEGER,
			workspace_id UUID NOT NULL,
			deployment_machine_type_id UUID
				CONSTRAINT plans_uq_deployment_machine_type_id
					UNIQUE
		);
	"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS billable_service(
			id UUID CONSTRAINT billable_service_pk PRIMARY KEY,
			plan_id UUID NOT NULL,
			workspace_id UUID NOT NULL,
			price DECIMAL(10, 2) NOT NULL,
			quantity INTEGER,
			product_info_id UUID NOT NULL,
			total_price DECIMAL(10, 2) NOT NULL GENERATED ALWAYS AS (price * quantity) STORED,
			resource_id UUID NOT NULL,
			date TIMESTAMPTZ NOT NULL,
			active BOOLEAN NOT NULL
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
		ALTER TABLE billable_service
		ADD CONSTRAINT billable_service_fk_resource_id
		FOREIGN KEY(resource_id) REFERENCES resource(id);
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

	initialize_product_table(connection).await?;

	Ok(())
}

async fn initialize_product_table(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	let product_id = generate_new_product_id(connection).await?;

	query!(
		r#"
		INSERT INTO 
			product_info(
				id, 
				name, 
				description
			)
		VALUES(
			$1,
			'deployment',
			NULL
		);
		"#,
		product_id as _
	)
	.execute(&mut *connection)
	.await?;

	let product_id = generate_new_product_id(connection).await?;

	query!(
		r#"
		INSERT INTO 
			product_info(
				id, 
				name, 
				description
			)
		VALUES(
			$1,
			'static_site',
			NULL
		);
		"#,
		product_id as _
	)
	.execute(&mut *connection)
	.await?;

	let product_id = generate_new_product_id(connection).await?;

	query!(
		r#"
		INSERT INTO 
			product_info(
				id, 
				name, 
				description
			)
		VALUES(
			$1,
			'managed_database',
			NULL
		);
		"#,
		product_id as _
	)
	.execute(&mut *connection)
	.await?;

	let product_id = generate_new_product_id(connection).await?;

	query!(
		r#"
		INSERT INTO 
			product_info(
				id, 
				name, 
				description
			)
		VALUES(
			$1,
			'managed_url',
			NULL
		);
		"#,
		product_id as _
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn create_resource_limit(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	max_resources: u32,
) -> Result<(), sqlx::Error> {
	let resource_limit_id = generate_new_resource_limit_id(connection).await?;

	query!(
		r#"
		INSERT INTO 
			resource_limits(
				id, 
				workspace_id, 
				max_resources
			)
		VALUES(
			$1,
			$2,
			$3
		);
		"#,
		resource_limit_id as _,
		workspace_id as _,
		max_resources as _
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn get_product_info_by_name(
	connection: &mut <Database as sqlx::Database>::Connection,
	name: &str,
) -> Result<Option<ProductInfo>, sqlx::Error> {
	query_as!(
		ProductInfo,
		r#"
		SELECT
			id as "id!: _",
			name,
			description
		FROM
			product_info
		WHERE
			name = $1;
		"#,
		name as _
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn create_product_limit(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	product_info_id: &Uuid,
	product_limit: u32,
) -> Result<(), sqlx::Error> {
	let product_id = generate_new_billable_service_id(connection).await?;

	query!(
		r#"
		INSERT INTO 
			product_limits(
				id,
				product_info_id,
				max_limit,
				workspace_id
			)
		VALUES(
			$1,
			$2,
			$3,
			$4
		);
	"#,
		product_id as _,
		product_info_id as _,
		product_limit as _,
		workspace_id as _
	)
	.execute(&mut *connection)
	.await?;
	Ok(())
}

pub async fn create_billable_service(
	connection: &mut <Database as sqlx::Database>::Connection,
	plan_id: &Uuid,
	workspace_id: &Uuid,
	price: f64,
	quantity: Option<i32>,
	product_info_id: &Uuid,
	resource_id: &Uuid,
	date: DateTime<Utc>,
	active: bool,
) -> Result<Uuid, sqlx::Error> {
	let billable_service_id =
		generate_new_billable_service_id(connection).await?;

	query!(
		r#"
		INSERT INTO 
			billable_service(
				id,
				plan_id,
				workspace_id,
				price,
				quantity,
				product_info_id,
				resource_id,
				date,
				active
			)
		VALUES(
			$1,
			$2,
			$3,
			$4,
			$5,
			$6,
			$7,
			$8,
			$9

		);
		"#,
		billable_service_id as _,
		plan_id as _,
		workspace_id as _,
		price as _,
		quantity as _,
		product_info_id as _,
		resource_id as _,
		date as _,
		active,
	)
	.execute(&mut *connection)
	.await?;

	Ok(billable_service_id)
}

pub async fn get_plan_by_deployment_machine_type(
	connection: &mut <Database as sqlx::Database>::Connection,
	machine_type_id: &Uuid,
) -> Result<Option<Plans>, sqlx::Error> {
	query_as!(
		Plans,
		r#"
		SELECT
			id as "id!: _",
			name,
			description,
			plan_type as "plan_type!: _",
			product_info_id as "product_info_id!: _",
			price as "price!: _",
			quantity,
			workspace_id as "workspace_id!: _"
		FROM
			plans
		WHERE
			deployment_machine_type_id = $1;
		"#,
		machine_type_id as _
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_billable_services(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	start_date: DateTime<Utc>,
	end_date: DateTime<Utc>,
) -> Result<Vec<BillableServiceUsage>, sqlx::Error> {
	query_as!(
		BillableServiceUsage,
		r#"
		SELECT
			id as "id!: _",
			plan_id as "plan_id!: _",
			workspace_id as "workspace_id!: _",
			price as "price!: _",
			total_price as "total_price!: _",
			quantity,
			product_info_id as "product_info_id!: _",
			resource_id as "resource_id!: _",
			date as "date!: _",
			active
		FROM
			billable_service
		WHERE
			workspace_id = $1 AND
			date BETWEEN $2 AND $3
		GROUP BY 
			id, resource_id
		ORDER BY
			date ASC;
		"#,
		workspace_id as _,
		start_date as _,
		end_date as _,
	)
	.fetch_all(&mut *connection)
	.await
}

async fn generate_new_resource_limit_id(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Uuid, sqlx::Error> {
	loop {
		let uuid = Uuid::new_v4();

		let exists = query!(
			r#"
			SELECT
				*
			FROM
				resource_limits
			WHERE
				id = $1;
				"#,
			uuid as _
		)
		.fetch_optional(&mut *connection)
		.await?
		.is_some();

		if !exists {
			break Ok(uuid);
		}
	}
}

async fn generate_new_plan_id(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Uuid, sqlx::Error> {
	loop {
		let uuid = Uuid::new_v4();

		let exists = query!(
			r#"
			SELECT
				id
			FROM
				plans
			WHERE
				id = $1;
			"#,
			uuid as _
		)
		.fetch_optional(&mut *connection)
		.await?
		.is_some();

		if !exists {
			break Ok(uuid);
		}
	}
}

async fn generate_new_product_id(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Uuid, sqlx::Error> {
	loop {
		let uuid = Uuid::new_v4();

		let exists = query!(
			r#"
			SELECT
				*
			FROM
				product_info
			WHERE
				id = $1;
			"#,
			uuid as _
		)
		.fetch_optional(&mut *connection)
		.await?
		.is_some();

		if !exists {
			break Ok(uuid);
		}
	}
}

async fn generate_new_billable_service_id(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Uuid, sqlx::Error> {
	loop {
		let uuid = Uuid::new_v4();

		let exists = query!(
			r#"
			SELECT
				*
			FROM
				billable_service
			WHERE
				id = $1;
			"#,
			uuid as _
		)
		.fetch_optional(&mut *connection)
		.await?
		.is_some();

		if !exists {
			break Ok(uuid);
		}
	}
}
