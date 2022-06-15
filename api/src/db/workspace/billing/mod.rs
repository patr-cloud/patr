use api_macros::{query, query_as};
use api_models::{
	models::workspace::billing::PaymentStatus,
	utils::{DateTime, Uuid},
};
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
	Card,
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
	pub workspace_id: Uuid,
}

pub struct BillableServiceUsage {
	pub id: Uuid,
	pub plan_id: Uuid,
	pub workspace_id: Uuid,
	pub price: f64,
	pub quantity: Option<i32>,
	pub product_info_id: Uuid,
	pub total_price: f64,
	pub resource_id: Uuid,
	pub date: DateTime<Utc>,
	pub active: bool,
	pub deployment_machine_type: Option<Uuid>,
	pub resource_type: String,
}

pub struct PaymentMethod {
	pub id: String,
	pub method_type: PaymentMethodType,
	pub workspace_id: Uuid,
	pub status: PaymentStatus,
}

pub struct ActiveCoupon {
	pub coupon_id: Uuid,
	pub workspace_id: Uuid,
	pub remaining_usage: i32,
}
pub struct Coupon {
	pub id: Uuid,
	pub name: String,
	pub description: Option<String>,
	pub credits: f64,
	pub valid_from: DateTime<Utc>,
	pub valid_till: DateTime<Utc>,
	pub num_usage: i32,
}

pub struct ProductLimit {
	pub id: Uuid,
	pub product_info_id: Uuid,
	pub max_limit: i32,
	pub workspace_id: Uuid,
}

pub struct ResourceLimit {
	pub id: Uuid,
	pub max_resources: i32,
	pub workspace_id: Uuid,
}

pub struct Credits {
	pub credits: f64,
}

pub struct ActivePlan {
	pub product_id: Uuid,
	pub plan_id: Uuid,
	pub workspace_id: Uuid,
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
			'card'
		);
	"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE PAYMENT_STATUS AS ENUM(
			'equires_payment_method',
			'requires_confirmation',
			'requires_action',
			'processing',
			'canceled',
			'succeeded'
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
			price DECIMAL(15, 6) NOT NULL,
			quantity INTEGER,
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
			plan_id UUID NOT NULL,
			workspace_id UUID NOT NULL,
			price DECIMAL(15, 6) NOT NULL,
			quantity INTEGER,
			product_info_id UUID NOT NULL,
			total_price DECIMAL(15, 6) NOT NULL GENERATED ALWAYS AS (price * quantity) STORED,
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
			product_info_id UUID,
			transaction_type TRANSACTION_TYPE NOT NULL,
			workspace_id UUID NOT NULL,
			date TIMESTAMPTZ NOT NULL,
			amount DECIMAL(15, 6) NOT NULL,
			quantity INTEGER,
			coupon_id UUID,
			plan_id UUID,
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
			name TEXT NOT NULL UNIQUE,
			description TEXT,
			credits DECIMAL(15, 6) NOT NULL,
			valid_from TIMESTAMPTZ NOT NULL,
			valid_till TIMESTAMPTZ NOT NULL,
			num_usage INTEGER NOT NULL	
		);
	"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS resource_limits(
			id UUID CONSTRAINT resource_limits_pk PRIMARY KEY,
			workspace_id UUID NOT NULL
				CONSTRAINT resource_limits_uq_workspace_id UNIQUE,
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
			workspace_id UUID NOT NULL,
			CONSTRAINT product_limits_uq_product_info_id_workspace_id UNIQUE (product_info_id, workspace_id)
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
			workspace_id UUID NOT NULL,
			status PAYMENT_STATUS NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS active_coupons(
			coupon_id UUID NOT NULL,
			workspace_id UUID NOT NULL,
			remaining_usage INTEGER NOT NULL,
			CONSTRAINT active_coupons_pk PRIMARY KEY (coupon_id, workspace_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS active_plans(
			product_id UUID NOT NULL,
			plan_id UUID NOT NULL,
			workspace_id UUID NOT NULL,
			CONSTRAINT active_plans_pk PRIMARY KEY (product_id, workspace_id)
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
		ALTER TABLE active_coupons
		ADD CONSTRAINT active_coupons_fk_id
		FOREIGN KEY(coupon_id) REFERENCES coupons(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE active_coupons
		ADD CONSTRAINT active_coupons_fk_workspace_id
		FOREIGN KEY(workspace_id) REFERENCES workspace(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE active_plans
		ADD CONSTRAINT active_plans_fk_id_workspace_id
		FOREIGN KEY(plan_id) REFERENCES plans(id),
		ADD CONSTRAINT active_plans_fk_workspace_id
		FOREIGN KEY(workspace_id) REFERENCES workspace(id),
		ADD CONSTRAINT active_plans_fk_product_id
		FOREIGN KEY(product_id) REFERENCES product_info(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE MATERIALIZED VIEW credits
		AS
			SELECT COALESCE(
					SUM(
						CASE debit
						WHEN 'true' THEN 
							-amount
						ELSE 
							amount
						END
					)
				) AS credits,
				workspace_id
			FROM transactions
			WHERE
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

	let deployment_product_id = generate_new_product_id(connection).await?;
	create_new_product(connection, &deployment_product_id, "deployment", None)
		.await?;

	let static_site_product_id = generate_new_product_id(connection).await?;
	create_new_product(
		connection,
		&static_site_product_id,
		"static-site",
		None,
	)
	.await?;

	let managed_database_product_id =
		generate_new_product_id(connection).await?;
	create_new_product(
		connection,
		&managed_database_product_id,
		"managed-database",
		None,
	)
	.await?;

	let managed_url_product_id = generate_new_product_id(connection).await?;
	create_new_product(
		connection,
		&managed_url_product_id,
		"managed-url",
		None,
	)
	.await?;

	let secret_product_id = generate_new_product_id(connection).await?;
	create_new_product(connection, &secret_product_id, "secret", None).await?;

	let secret_product_id = generate_new_product_id(connection).await?;
	create_new_product(connection, &secret_product_id, "domain", None).await?;

	Ok(())
}

pub async fn create_new_product(
	connection: &mut <Database as sqlx::Database>::Connection,
	id: &Uuid,
	name: &str,
	description: Option<&str>,
) -> Result<(), sqlx::Error> {
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
			$2,
			$3
		);
		"#,
		id as _,
		name,
		description
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn create_resource_limit(
	connection: &mut <Database as sqlx::Database>::Connection,
	id: &Uuid,
	workspace_id: &Uuid,
	max_resources: u32,
) -> Result<(), sqlx::Error> {
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
		id as _,
		workspace_id as _,
		max_resources as _
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn create_new_plan(
	connection: &mut <Database as sqlx::Database>::Connection,
	id: &Uuid,
	name: &str,
	description: Option<&str>,
	plan_type: &PlanType,
	product_info_id: &Uuid,
	price: f64,
	quantity: Option<i32>,
	workspace_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO 
			plans(
				id, 
				name, 
				description,
				plan_type,
				product_info_id,
				price,
				quantity,
				workspace_id
			)
		VALUES(
			$1,
			$2,
			$3,
			$4,
			$5,
			$6,
			$7,
			$8
		);
		"#,
		id as _,
		name,
		description as _,
		plan_type as _,
		product_info_id as _,
		price as _,
		quantity as _,
		workspace_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
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
	let id = generate_new_product_limit_id(connection).await?;

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
		id as _,
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
	id: &Uuid,
	plan_id: &Uuid,
	workspace_id: &Uuid,
	price: f64,
	quantity: Option<i32>,
	product_info_id: &Uuid,
	resource_id: &Uuid,
	date: DateTime<Utc>,
	active: bool,
) -> Result<(), sqlx::Error> {
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
		id as _,
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

	Ok(())
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
			billable_service.id as "id!: _",
			billable_service.plan_id as "plan_id!: _",
			billable_service.workspace_id as "workspace_id!: _",
			billable_service.price::FLOAT8 as "price!: _",
			billable_service.quantity,
			billable_service.product_info_id as "product_info_id!: _",
			billable_service.total_price::FLOAT8 as "total_price!: _",
			billable_service.resource_id as "resource_id!: _",
			billable_service.date as "date!: _",
			billable_service.active,
			deployment.machine_type as "deployment_machine_type: _",
			resource_type.name as "resource_type!: _"
		FROM
			billable_service
		INNER JOIN
			deployment
		ON
			billable_service.resource_id = deployment.id
		INNER JOIN
			resource
		ON
			billable_service.resource_id = resource.id
		INNER JOIN
			resource_type
		ON
			resource.resource_type_id = resource_type.id
		WHERE
			billable_service.workspace_id = $1 AND
			date BETWEEN $2 AND $3
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

pub async fn add_payment_method_info(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	payment_method_id: &str,
	status: PaymentStatus,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO 
			payment_method(
				id,
				method_type,
				workspace_id,
				status
			)
		VALUES(
			$1,
			'card',
			$2,
			$3
		);
		"#,
		payment_method_id as _,
		workspace_id as _,
		status as _,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn get_payment_methods_for_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<Vec<PaymentMethod>, sqlx::Error> {
	query_as!(
		PaymentMethod,
		r#"
		SELECT
			id as "id!: _",
			method_type as "method_type!: _",
			workspace_id as "workspace_id!: _",
			status as "status!: _"
		FROM
			payment_method
		WHERE
			workspace_id = $1;
		"#,
		workspace_id as _
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn create_transaction(
	connection: &mut <Database as sqlx::Database>::Connection,
	id: &Uuid,
	product_info_id: Option<&Uuid>,
	transaction_type: &TransactionType,
	plan_id: Option<&Uuid>,
	amount: f64,
	quantity: Option<i32>,
	workspace_id: &Uuid,
	date: &DateTime<Utc>,
	coupon_id: Option<&Uuid>,
	debit: bool,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO 
			transactions(
				id,
				product_info_id,
				transaction_type,
				date,
				workspace_id,
				amount,
				quantity,
				coupon_id,
				plan_id,
				debit
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
			$9,
			$10
		);
		"#,
		id as _,
		product_info_id as _,
		transaction_type as _,
		date as _,
		workspace_id as _,
		amount as _,
		quantity as _,
		coupon_id as _,
		plan_id as _,
		debit as _,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn generate_new_resource_limit_id(
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

pub async fn generate_new_plan_id(
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

pub async fn generate_new_product_id(
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

pub async fn generate_new_billable_service_id(
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

pub async fn generate_new_transaction_id(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Uuid, sqlx::Error> {
	loop {
		let uuid = Uuid::new_v4();

		let exists = query!(
			r#"
			SELECT
				id
			FROM
				transactions
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

pub async fn generate_new_product_limit_id(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Uuid, sqlx::Error> {
	loop {
		let uuid = Uuid::new_v4();

		let exists = query!(
			r#"
			SELECT
				*
			FROM
				product_limits
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

pub async fn update_product_limits(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	product_info_id: &Uuid,
	limit: i32,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			product_limits
		SET
			max_limit = $1
		WHERE
			product_info_id = $2 AND
			workspace_id = $3;
		"#,
		limit,
		product_info_id as _,
		workspace_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_payment_method_info(
	connection: &mut <Database as sqlx::Database>::Connection,
	payment_method_id: &str,
) -> Result<Option<PaymentMethod>, sqlx::Error> {
	query_as!(
		PaymentMethod,
		r#"
		SELECT
			id as "id!: _",
			method_type as "method_type!: _",
			workspace_id as "workspace_id!: _",
			status as "status!: _"
		FROM
			payment_method
		WHERE
			id = $1;
		"#,
		payment_method_id as _,
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn delete_payment_method(
	connection: &mut <Database as sqlx::Database>::Connection,
	payment_method_id: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			payment_method
		WHERE
			id = $1;
		"#,
		payment_method_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn add_coupon_to_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	coupon_id: &Uuid,
	remaining_usage: &i32,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			active_coupons(
				workspace_id, 
				coupon_id, 
				remaining_usage
			)
		VALUES
			($1, $2, $3);
			"#,
		workspace_id as _,
		coupon_id as _,
		remaining_usage as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_active_coupon_by_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	coupon_id: &Uuid,
	workspace_id: &Uuid,
) -> Result<Option<ActiveCoupon>, sqlx::Error> {
	query_as!(
		ActiveCoupon,
		r#"
		SELECT
			coupon_id as "coupon_id!: _",
			workspace_id as "workspace_id!: _",
			remaining_usage as "remaining_usage!: _"
		FROM
			active_coupons
		WHERE
			coupon_id = $1 AND
			workspace_id = $2;
			"#,
		coupon_id as _,
		workspace_id as _,
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn update_active_coupon_usage(
	connection: &mut <Database as sqlx::Database>::Connection,
	id: &Uuid,
	workspace_id: &Uuid,
	num_usage: i32,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			active_coupons
		SET
			remaining_usage = $1
		WHERE
			coupon_id = $2 AND
			workspace_id = $3;
		"#,
		num_usage as _,
		id as _,
		workspace_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_coupon_by_name(
	connection: &mut <Database as sqlx::Database>::Connection,
	name: &str,
) -> Result<Option<Coupon>, sqlx::Error> {
	query_as!(
		Coupon,
		r#"
		SELECT
			id as "id!: _",
			name,
			description,
			credits::FLOAT8 as "credits!: _",
			valid_from as "valid_from!: _",
			valid_till as "valid_till!: _",
			num_usage as "num_usage!: _"
		FROM
			coupons
		WHERE
			name = $1;
		"#,
		name as _,
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_resource_limit(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<Option<ResourceLimit>, sqlx::Error> {
	query_as!(
		ResourceLimit,
		r#"
		SELECT
			id as "id!: _",
			workspace_id as "workspace_id!: _",
			max_resources as "max_resources!: _"
		FROM
			resource_limits
		WHERE
			workspace_id = $1;
		"#,
		workspace_id as _,
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_product_limit(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	product_info_id: &Uuid,
) -> Result<Option<ProductLimit>, sqlx::Error> {
	query_as!(
		ProductLimit,
		r#"
		SELECT
			id as "id!: _",
			product_info_id as "product_info_id!: _",
			workspace_id as "workspace_id!: _",
			max_limit as "max_limit!: _"
		FROM
			product_limits
		WHERE
			workspace_id = $1 AND
			product_info_id = $2;
		"#,
		workspace_id as _,
		product_info_id as _,
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_credit_balance(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<Credits, sqlx::Error> {
	query_as!(
		Credits,
		r#"
		SELECT
			credits::FLOAT8 as "credits!: _"
		FROM
			credits
		WHERE
			workspace_id = $1;
		"#,
		workspace_id as _,
	)
	.fetch_one(&mut *connection)
	.await
}

pub async fn get_active_plan_for_product_in_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	product_info_id: &Uuid,
) -> Result<Option<ActivePlan>, sqlx::Error> {
	query_as!(
		ActivePlan,
		r#"
		SELECT
			product_id as "product_id!: _",
			plan_id as "plan_id!: _",
			workspace_id as "workspace_id!: _"
		FROM
			active_plans
		WHERE
			workspace_id = $1 AND
			product_id = $2;
		"#,
		workspace_id as _,
		product_info_id as _,
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_plan_by_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	id: &Uuid,
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
			price::FLOAT8 as "price!: _",
			quantity,
			workspace_id as "workspace_id!: _"
		FROM
			plans
		WHERE
			id = $1;
		"#,
		id as _,
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_plan_by_name(
	connection: &mut <Database as sqlx::Database>::Connection,
	name: &str,
	workspace_id: &Uuid,
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
			price::FLOAT8 as "price!: _",
			quantity,
			workspace_id as "workspace_id!: _"
		FROM
			plans
		WHERE
			name = $1 AND
			workspace_id = $2;
		"#,
		name,
		workspace_id as _,
	)
	.fetch_optional(&mut *connection)
	.await
}
