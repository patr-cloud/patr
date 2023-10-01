use std::fmt;

use api_macros::query;
use api_models::{
	models::workspace::billing::{PaymentStatus, TotalAmount, TransactionType},
	utils::Uuid,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::query_as;

use super::LegacyManagedDatabasePlan;
use crate::Database;

pub struct DeploymentPaymentHistory {
	pub workspace_id: Uuid,
	pub deployment_id: Uuid,
	pub machine_type: Uuid,
	pub num_instance: i32,
	pub start_time: DateTime<Utc>,
	pub stop_time: Option<DateTime<Utc>>,
}

pub struct VolumePaymentHistory {
	pub workspace_id: Uuid,
	pub volume_id: Uuid,
	pub storage: i64,
	pub number_of_volumes: i32,
	pub start_time: DateTime<Utc>,
	pub stop_time: Option<DateTime<Utc>>,
}

pub struct StaticSitesPaymentHistory {
	pub workspace_id: Uuid,
	pub static_site_plan: StaticSitePlan,
	pub start_time: DateTime<Utc>,
	pub stop_time: Option<DateTime<Utc>>,
}

pub struct ManagedDatabasePaymentHistory {
	pub workspace_id: Uuid,
	pub database_id: Uuid,
	pub db_plan_id: Option<Uuid>,
	pub legacy_db_plan: Option<LegacyManagedDatabasePlan>,
	pub start_time: DateTime<Utc>,
	pub deletion_time: Option<DateTime<Utc>>,
}

pub struct ManagedUrlPaymentHistory {
	pub workspace_id: Uuid,
	pub url_count: i32,
	pub start_time: DateTime<Utc>,
	pub stop_time: Option<DateTime<Utc>>,
}

pub struct SecretPaymentHistory {
	pub workspace_id: Uuid,
	pub secret_count: i32,
	pub start_time: DateTime<Utc>,
	pub stop_time: Option<DateTime<Utc>>,
}

pub struct DockerRepoPaymentHistory {
	pub workspace_id: Uuid,
	pub storage: i64,
	pub start_time: DateTime<Utc>,
	pub stop_time: Option<DateTime<Utc>>,
}

pub struct DomainPaymentHistory {
	pub workspace_id: Uuid,
	pub domain_plan: DomainPlan,
	pub start_time: DateTime<Utc>,
	pub stop_time: Option<DateTime<Utc>>,
}

pub struct Transaction {
	pub id: Uuid,
	pub month: i32,
	pub amount_in_cents: i64,
	pub payment_intent_id: Option<String>,
	pub date: DateTime<Utc>,
	pub workspace_id: Uuid,
	pub transaction_type: TransactionType,
	pub payment_status: PaymentStatus,
	pub description: Option<String>,
}

pub struct PaymentMethod {
	pub payment_method_id: String,
	pub workspace_id: Uuid,
}

#[derive(
	Debug, sqlx::Type, Serialize, Deserialize, PartialEq, Eq, Hash, Clone,
)]
#[sqlx(type_name = "STATIC_SITE_PLAN", rename_all = "lowercase")]
pub enum StaticSitePlan {
	Free,
	Pro,
	Unlimited,
}

impl fmt::Display for StaticSitePlan {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			StaticSitePlan::Free => write!(f, "free"),
			StaticSitePlan::Pro => write!(f, "pro"),
			StaticSitePlan::Unlimited => write!(f, "unlimited"),
		}
	}
}

#[derive(
	Debug, sqlx::Type, Serialize, Deserialize, PartialEq, Eq, Hash, Clone,
)]
#[sqlx(type_name = "DOMAIN_PLAN", rename_all = "lowercase")]
pub enum DomainPlan {
	Free,
	Unlimited,
}

impl fmt::Display for DomainPlan {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			DomainPlan::Free => write!(f, "free"),
			DomainPlan::Unlimited => write!(f, "unlimited"),
		}
	}
}

pub async fn initialize_billing_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Initializing billing tables");

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
		CREATE TABLE deployment_payment_history(
			workspace_id UUID NOT NULL,
			deployment_id UUID NOT NULL,
			machine_type UUID NOT NULL,
			num_instance INTEGER NOT NULL,
			start_time TIMESTAMPTZ NOT NULL,
			stop_time TIMESTAMPTZ
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE static_sites_payment_history(
			workspace_id UUID NOT NULL,
			static_site_plan STATIC_SITE_PLAN NOT NULL,
			start_time TIMESTAMPTZ NOT NULL,
			stop_time TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE managed_database_payment_history(
			workspace_id UUID NOT NULL,
			database_id UUID NOT NULL,
			db_plan_id UUID
				CONSTRAINT managed_database_payment_history_fk_db_plan_id
					REFERENCES managed_database_plan(id),
			legacy_db_plan LEGACY_MANAGED_DATABASE_PLAN,
			start_time TIMESTAMPTZ NOT NULL,
			deletion_time TIMESTAMPTZ,
			CONSTRAINT managed_database_payment_history_chk_legacy_db_plan
				CHECK(
					(
						db_plan_id IS NULL AND
						legacy_db_plan IS NOT NULL
					) OR (
						db_plan_id IS NOT NULL AND
						legacy_db_plan IS NULL
					)
				)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE managed_url_payment_history(
			workspace_id UUID NOT NULL,
			url_count INTEGER NOT NULL,
			start_time TIMESTAMPTZ NOT NULL,
			stop_time TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE secrets_payment_history(
			workspace_id UUID NOT NULL,
			secret_count INTEGER NOT NULL,
			start_time TIMESTAMPTZ NOT NULL,
			stop_time TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE docker_repo_payment_history(
			workspace_id UUID NOT NULL,
			storage BIGINT NOT NULL,
			start_time TIMESTAMPTZ NOT NULL,
			stop_time TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE domain_payment_history(
			workspace_id UUID NOT NULL,
			domain_plan DOMAIN_PLAN NOT NULL,
			start_time TIMESTAMPTZ NOT NULL,
			stop_time TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE volume_payment_history(
			workspace_id UUID NOT NULL,
			volume_id UUID NOT NULL,
			storage BIGINT NOT NULL,
			number_of_volumes INTEGER NOT NULL,
			start_time TIMESTAMPTZ NOT NULL,
			stop_time TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE payment_method(
			payment_method_id TEXT CONSTRAINT payment_method_pk PRIMARY KEY,
			workspace_id UUID NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE TRANSACTION_TYPE as ENUM(
			'bill',
			'credits',
			'payment'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE PAYMENT_STATUS as ENUM(
			'success',
			'failed',
			'pending'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE transaction(
			id UUID CONSTRAINT transaction_pk PRIMARY KEY,
			month INTEGER NOT NULL,
			amount_in_cents BIGINT NOT NULL
				CONSTRAINT transaction_chk_amount_in_cents_positive
					CHECK (amount_in_cents >= 0),
			payment_intent_id TEXT,
			date TIMESTAMPTZ NOT NULL,
			workspace_id UUID NOT NULL,
			transaction_type TRANSACTION_TYPE NOT NULL,
			payment_status PAYMENT_STATUS NOT NULL
				CONSTRAINT transaction_payment_status_check CHECK(
					(
						payment_status = 'success' AND
						transaction_type = 'bill'
					) OR
					(
						transaction_type != 'bill'
					)
				),
			description TEXT
				CONSTRAINT transaction_description_check CHECK(
					(
						transaction_type = 'credits' AND
						description IS NOT NULL
					) OR (
						transaction_type != 'credits'
					)
				)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn initialize_billing_constraints(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Finishing up billing tables initialization");

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

	query!(
		r#"
		ALTER TABLE payment_method
		ADD CONSTRAINT payment_method_workspace_id_fk
		FOREIGN KEY (workspace_id) REFERENCES workspace(id)
		DEFERRABLE INITIALLY IMMEDIATE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE transaction
		ADD CONSTRAINT transaction_workspace_id_fk
		FOREIGN KEY (workspace_id) REFERENCES workspace(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn get_all_deployment_usage(
	connection: &mut DatabaseConnection,
	workspace_id: &Uuid,
	month_start_date: &DateTime<Utc>,
	till_date: &DateTime<Utc>,
) -> Result<Vec<DeploymentPaymentHistory>, sqlx::Error> {
	query_as!(
		DeploymentPaymentHistory,
		r#"
		SELECT
			workspace_id as "workspace_id: _",
			deployment_id as "deployment_id: _",
			machine_type as "machine_type: _",
			num_instance as "num_instance: _",
			start_time as "start_time: _",
			stop_time as "stop_time: _"
		FROM
			deployment_payment_history
		WHERE
			workspace_id = $1 AND
			(start_time, COALESCE(stop_time, NOW())) OVERLAPS ($2, $3)
		ORDER BY
			start_time ASC;
		"#,
		workspace_id as _,
		month_start_date as _,
		till_date as _,
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn get_deployment_usage(
	connection: &mut DatabaseConnection,
	deployment_id: &Uuid,
	month_start_date: &DateTime<Utc>,
	till_date: &DateTime<Utc>,
) -> Result<Vec<DeploymentPaymentHistory>, sqlx::Error> {
	query_as!(
		DeploymentPaymentHistory,
		r#"
		SELECT
			workspace_id as "workspace_id: _",
			deployment_id as "deployment_id: _",
			machine_type as "machine_type: _",
			num_instance as "num_instance: _",
			start_time as "start_time: _",
			stop_time as "stop_time: _"
		FROM
			deployment_payment_history
		WHERE
			deployment_id = $1 AND
			(start_time, COALESCE(stop_time, NOW())) OVERLAPS ($2, $3)
		ORDER BY
			start_time ASC;
		"#,
		deployment_id as _,
		month_start_date as _,
		till_date as _,
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn get_all_volume_usage(
	connection: &mut DatabaseConnection,
	workspace_id: &Uuid,
	month_start_date: &DateTime<Utc>,
	till_date: &DateTime<Utc>,
) -> Result<Vec<VolumePaymentHistory>, sqlx::Error> {
	query_as!(
		VolumePaymentHistory,
		r#"
		SELECT
			workspace_id as "workspace_id: _",
			volume_id as "volume_id: _",
			storage,
			number_of_volumes,
			start_time as "start_time: _",
			stop_time as "stop_time: _"
		FROM
			volume_payment_history
		WHERE
			workspace_id = $1 AND
			(
				stop_time IS NULL OR
				stop_time > $2
			) AND
			start_time < $3;
		"#,
		workspace_id as _,
		month_start_date as _,
		till_date as _,
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn get_all_managed_database_usage(
	connection: &mut DatabaseConnection,
	workspace_id: &Uuid,
	start_date: &DateTime<Utc>,
	till_date: &DateTime<Utc>,
) -> Result<Vec<ManagedDatabasePaymentHistory>, sqlx::Error> {
	query_as!(
		ManagedDatabasePaymentHistory,
		r#"
		SELECT
			workspace_id as "workspace_id: _",
			database_id as "database_id: _",
			legacy_db_plan as "legacy_db_plan: _",
			db_plan_id as "db_plan_id: _",
			start_time as "start_time: _",
			deletion_time as "deletion_time: _"
		FROM
			managed_database_payment_history
		WHERE
			workspace_id = $1 AND
			(start_time, COALESCE(deletion_time, NOW())) OVERLAPS ($2, $3)
		ORDER BY
			start_time ASC;
		"#,
		workspace_id as _,
		start_date as _,
		till_date as _,
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn get_all_static_site_usages(
	connection: &mut DatabaseConnection,
	workspace_id: &Uuid,
	start_date: &DateTime<Utc>,
	till_date: &DateTime<Utc>,
) -> Result<Vec<StaticSitesPaymentHistory>, sqlx::Error> {
	query_as!(
		StaticSitesPaymentHistory,
		r#"
		SELECT
			workspace_id as "workspace_id: _",
			static_site_plan as "static_site_plan: _",
			start_time as "start_time: _",
			stop_time as "stop_time: _"
		FROM
			static_sites_payment_history
		WHERE
			workspace_id = $1 AND
			(start_time, COALESCE(stop_time, NOW())) OVERLAPS ($2, $3)
		ORDER BY
			start_time ASC;
		"#,
		workspace_id as _,
		start_date as _,
		till_date as _
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn get_all_managed_url_usages(
	connection: &mut DatabaseConnection,
	workspace_id: &Uuid,
	start_date: &DateTime<Utc>,
	till_date: &DateTime<Utc>,
) -> Result<Vec<ManagedUrlPaymentHistory>, sqlx::Error> {
	query_as!(
		ManagedUrlPaymentHistory,
		r#"
		SELECT
			workspace_id as "workspace_id: _",
			url_count as "url_count: _",
			start_time as "start_time: _",
			stop_time as "stop_time: _"
		FROM
			managed_url_payment_history
		WHERE
			workspace_id = $1 AND
			(start_time, COALESCE(stop_time, NOW())) OVERLAPS ($2, $3)
		ORDER BY
			start_time ASC;
		"#,
		workspace_id as _,
		start_date as _,
		till_date as _,
	)
	.fetch_all(&mut *connection)
	.await
}
pub async fn get_all_docker_repository_usages(
	connection: &mut DatabaseConnection,
	workspace_id: &Uuid,
	start_date: &DateTime<Utc>,
	till_date: &DateTime<Utc>,
) -> Result<Vec<DockerRepoPaymentHistory>, sqlx::Error> {
	query_as!(
		DockerRepoPaymentHistory,
		r#"
		SELECT
			workspace_id as "workspace_id: _",
			storage as "storage: _",
			start_time as "start_time: _",
			stop_time as "stop_time: _"
		FROM
			docker_repo_payment_history
		WHERE
			workspace_id = $1 AND
			(start_time, COALESCE(stop_time, NOW())) OVERLAPS ($2, $3)
		ORDER BY
			start_time ASC;
		"#,
		workspace_id as _,
		start_date as _,
		till_date as _,
	)
	.fetch_all(&mut *connection)
	.await
}
pub async fn get_all_domains_usages(
	connection: &mut DatabaseConnection,
	workspace_id: &Uuid,
	start_date: &DateTime<Utc>,
	till_date: &DateTime<Utc>,
) -> Result<Vec<DomainPaymentHistory>, sqlx::Error> {
	query_as!(
		DomainPaymentHistory,
		r#"
		SELECT
			workspace_id as "workspace_id: _",
			domain_plan as "domain_plan: _",
			start_time as "start_time: _",
			stop_time as "stop_time: _"
		FROM
			domain_payment_history
		WHERE
			workspace_id = $1 AND
			(start_time, COALESCE(stop_time, NOW())) OVERLAPS ($2, $3)
		ORDER BY
			start_time ASC;
		"#,
		workspace_id as _,
		start_date as _,
		till_date as _,
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn get_all_secrets_usages(
	connection: &mut DatabaseConnection,
	workspace_id: &Uuid,
	month_start_date: &DateTime<Utc>,
	till_date: &DateTime<Utc>,
) -> Result<Vec<SecretPaymentHistory>, sqlx::Error> {
	query_as!(
		SecretPaymentHistory,
		r#"
		SELECT
			workspace_id as "workspace_id: _",
			secret_count as "secret_count: _",
			start_time as "start_time: _",
			stop_time as "stop_time: _"
		FROM
			secrets_payment_history
		WHERE
			workspace_id = $1 AND
			(start_time, COALESCE(stop_time, NOW())) OVERLAPS ($2, $3)
		ORDER BY
			start_time ASC;
		"#,
		workspace_id as _,
		month_start_date as _,
		till_date as _,
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn create_transaction(
	connection: &mut DatabaseConnection,
	workspace_id: &Uuid,
	id: &Uuid,
	month: i32,
	amount_in_cents: u64,
	payment_intent_id: Option<&str>,
	date: &DateTime<Utc>,
	transaction_type: &TransactionType,
	payment_status: &PaymentStatus,
	description: Option<&str>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			transaction(
				id,
				month,
				amount_in_cents,
				payment_intent_id,
				date,
				workspace_id,
				transaction_type,
				payment_status,
				description
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
				$9
			);
		"#,
		id as _,
		month as _,
		amount_in_cents as i64,
		payment_intent_id as _,
		date as _,
		workspace_id as _,
		transaction_type as _,
		payment_status as _,
		description
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_transaction_status(
	connection: &mut DatabaseConnection,
	transaction_id: &Uuid,
	payment_status: &PaymentStatus,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			transaction
		SET
			payment_status = $1
		WHERE
			id = $2;
		"#,
		payment_status as _,
		transaction_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn generate_new_transaction_id(
	connection: &mut DatabaseConnection,
) -> Result<Uuid, sqlx::Error> {
	loop {
		let uuid = Uuid::new_v4();

		let exists = query!(
			r#"
			SELECT
				id
			FROM
				transaction
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

pub async fn get_payment_methods_for_workspace(
	connection: &mut DatabaseConnection,
	workspace_id: &Uuid,
) -> Result<Vec<PaymentMethod>, sqlx::Error> {
	query_as!(
		PaymentMethod,
		r#"
		SELECT
			payment_method_id,
			workspace_id as "workspace_id!: _"
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

pub async fn get_total_amount_in_cents_to_pay_for_workspace(
	connection: &mut DatabaseConnection,
	workspace_id: &Uuid,
) -> Result<TotalAmount, sqlx::Error> {
	query!(
		r#"
		SELECT
			SUM(
				CASE transaction_type
					WHEN 'bill' THEN
						amount_in_cents
					WHEN 'credits' THEN
						-amount_in_cents
					WHEN 'payment' THEN
						-amount_in_cents
					ELSE
						0
				END
			)::BIGINT as "total_amount"
		FROM
			transaction
		WHERE
			workspace_id = $1 AND
			payment_status = 'success';
		"#,
		workspace_id as _,
	)
	.fetch_one(&mut *connection)
	.await
	.map(|row| row.total_amount.unwrap_or(0i64))
	.map(|amount| amount.into())
}

pub async fn get_transaction_by_transaction_id(
	connection: &mut DatabaseConnection,
	workspace_id: &Uuid,
	transaction_id: &Uuid,
) -> Result<Option<Transaction>, sqlx::Error> {
	query_as!(
		Transaction,
		r#"
		SELECT
			id as "id: _",
			month,
			amount_in_cents,
			payment_intent_id,
			date as "date: _",
			workspace_id as "workspace_id: _",
			transaction_type as "transaction_type: _",
			payment_status as "payment_status: _",
			description
		FROM
			transaction
		WHERE
			workspace_id = $1 AND
			id = $2;
		"#,
		workspace_id as _,
		transaction_id as _,
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn start_deployment_usage_history(
	connection: &mut DatabaseConnection,
	workspace_id: &Uuid,
	deployment_id: &Uuid,
	machine_type: &Uuid,
	num_instance: i32,
	start_time: &DateTime<Utc>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			deployment_payment_history(
				workspace_id,
				deployment_id,
				machine_type,
				num_instance,
				start_time,
				stop_time
			)
		VALUES
			(
				$1,
				$2,
				$3,
				$4,
				$5,
				NULL
			);
		"#,
		workspace_id as _,
		deployment_id as _,
		machine_type as _,
		num_instance as _,
		start_time as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn stop_deployment_usage_history(
	connection: &mut DatabaseConnection,
	deployment_id: &Uuid,
	stop_time: &DateTime<Utc>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			deployment_payment_history
		SET
			stop_time = $1
		WHERE	
			deployment_id = $2 AND
			stop_time IS NULL;
		"#,
		stop_time as _,
		deployment_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn start_volume_usage_history(
	connection: &mut DatabaseConnection,
	workspace_id: &Uuid,
	volume_id: &Uuid,
	storage: u64,
	number_of_volume: u16,
	start_time: &DateTime<Utc>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			volume_payment_history(
				workspace_id,
				volume_id,
				storage,
				number_of_volumes,
				start_time,
				stop_time
			)
		VALUES
			(
				$1,
				$2,
				$3,
				$4,
				$5,
				NULL
			);
		"#,
		workspace_id as _,
		volume_id as _,
		storage as i64,
		number_of_volume as i32,
		start_time as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn stop_volume_usage_history(
	connection: &mut DatabaseConnection,
	volume_id: &Uuid,
	stop_time: &DateTime<Utc>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			volume_payment_history
		SET
			stop_time = $1
		WHERE	
			volume_id = $2 AND
			stop_time IS NULL;
		"#,
		stop_time as _,
		volume_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_volume_payment_history_by_volume_id(
	connection: &mut DatabaseConnection,
	volume_id: &Uuid,
) -> Result<Option<VolumePaymentHistory>, sqlx::Error> {
	query_as!(
		VolumePaymentHistory,
		r#"
		SELECT
			workspace_id as "workspace_id: _",
			volume_id as "volume_id: _",
			storage,
			number_of_volumes,
			start_time as "start_time: _",
			stop_time as "stop_time: _"
		FROM
			volume_payment_history
		WHERE
			volume_id = $1 AND
			stop_time IS NULL;
		"#,
		volume_id as _,
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn start_managed_database_usage_history(
	connection: &mut DatabaseConnection,
	workspace_id: &Uuid,
	database_id: &Uuid,
	db_plan_id: &Uuid,
	start_time: &DateTime<Utc>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			managed_database_payment_history(
				workspace_id,
				database_id,
				legacy_db_plan,
				db_plan_id,
				start_time,
				deletion_time
			)
		VALUES
			(
				$1,
				$2,
				NULL,
				$3,
				$4,
				NULL
			);
		"#,
		workspace_id as _,
		database_id as _,
		db_plan_id as _,
		start_time as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn stop_managed_database_usage_history(
	connection: &mut DatabaseConnection,
	database_id: &Uuid,
	deletion_time: &DateTime<Utc>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE 
			managed_database_payment_history
		SET
			deletion_time = $2
		WHERE
			database_id = $1 AND
			deletion_time IS NULL;
		"#,
		database_id as _,
		deletion_time as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_static_site_usage_history(
	connection: &mut DatabaseConnection,
	workspace_id: &Uuid,
	static_site_plan: &StaticSitePlan,
	update_time: &DateTime<Utc>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			static_sites_payment_history
		SET
			stop_time = $1
		WHERE
			workspace_id = $2 AND
			stop_time IS NULL;
		"#,
		update_time as _,
		workspace_id as _,
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		INSERT INTO
			static_sites_payment_history(
				workspace_id,
				static_site_plan,
				start_time,
				stop_time
			)
		VALUES
			(
				$1,
				$2,
				$3,
				NULL
			);
		"#,
		workspace_id as _,
		static_site_plan as _,
		update_time as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_managed_url_usage_history(
	connection: &mut DatabaseConnection,
	workspace_id: &Uuid,
	url_count: &i32,
	update_time: &DateTime<Utc>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			managed_url_payment_history
		SET
			stop_time = $1
		WHERE
			workspace_id = $2 AND
			stop_time IS NULL;
		"#,
		update_time as _,
		workspace_id as _,
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		INSERT INTO
			managed_url_payment_history(
				workspace_id,
				url_count,
				start_time,
				stop_time
			)
		VALUES
			(
				$1,
				$2,
				$3,
				NULL
			);
		"#,
		workspace_id as _,
		url_count,
		update_time as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_docker_repo_usage_history(
	connection: &mut DatabaseConnection,
	workspace_id: &Uuid,
	storage: &i64,
	update_time: &DateTime<Utc>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			docker_repo_payment_history
		SET
			stop_time = $1
		WHERE
			workspace_id = $2 AND
			stop_time IS NULL;
		"#,
		update_time as _,
		workspace_id as _,
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		INSERT INTO
			docker_repo_payment_history(
				workspace_id,
				storage,
				start_time,
				stop_time
			)
		VALUES
			(
				$1,
				$2,
				$3,
				NULL
			);
		"#,
		workspace_id as _,
		storage,
		update_time as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_secret_usage_history(
	connection: &mut DatabaseConnection,
	workspace_id: &Uuid,
	secret_count: &i32,
	update_time: &DateTime<Utc>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			secrets_payment_history
		SET
			stop_time = $1
		WHERE
			workspace_id = $2 AND
			stop_time IS NULL;
		"#,
		update_time as _,
		workspace_id as _,
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		INSERT INTO
			secrets_payment_history(
				workspace_id,
				secret_count,
				start_time,
				stop_time
			)
		VALUES
			(
				$1,
				$2,
				$3,
				NULL
			);
		"#,
		workspace_id as _,
		secret_count,
		update_time as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_domain_usage_history(
	connection: &mut DatabaseConnection,
	workspace_id: &Uuid,
	domain_plan: &DomainPlan,
	update_time: &DateTime<Utc>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			domain_payment_history
		SET
			stop_time = $1
		WHERE
			workspace_id = $2 AND
			stop_time IS NULL;
		"#,
		update_time as _,
		workspace_id as _,
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		INSERT INTO
			domain_payment_history(
				workspace_id,
				domain_plan,
				start_time,
				stop_time
			)
		VALUES
			(
				$1,
				$2,
				$3,
				NULL
			);
		"#,
		workspace_id as _,
		domain_plan as _,
		update_time as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_amount_due_for_workspace(
	connection: &mut DatabaseConnection,
	workspace_id: &Uuid,
	amount_due_in_cents: u64,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			workspace
		SET
			amount_due_in_cents = $1
		WHERE
			id = $2;
		"#,
		amount_due_in_cents as i64,
		workspace_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_payment_method_info(
	connection: &mut DatabaseConnection,
	payment_method_id: &str,
) -> Result<Option<PaymentMethod>, sqlx::Error> {
	query_as!(
		PaymentMethod,
		r#"
		SELECT 
			payment_method_id,
			workspace_id as "workspace_id: _"
		FROM
			payment_method
		WHERE
			payment_method_id = $1;
		"#,
		payment_method_id
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn delete_payment_method(
	connection: &mut DatabaseConnection,
	payment_method_id: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			payment_method
		WHERE
			payment_method_id = $1;
		"#,
		payment_method_id
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn add_payment_method_info(
	connection: &mut DatabaseConnection,
	workspace_id: &Uuid,
	payment_method_id: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO 
			payment_method(
				payment_method_id,
				workspace_id
			)
		VALUES(
			$1,
			$2
		);
		"#,
		payment_method_id,
		workspace_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_transactions_in_workspace(
	connection: &mut DatabaseConnection,
	workspace_id: &Uuid,
) -> Result<Vec<Transaction>, sqlx::Error> {
	query_as!(
		Transaction,
		r#"
		SELECT
			id as "id: _",
			month,
			amount_in_cents,
			payment_intent_id,
			date as "date: _",
			workspace_id as "workspace_id: _",
			transaction_type as "transaction_type!: _",
			payment_status as "payment_status!: _",
			description
		FROM
			transaction
		WHERE
			workspace_id = $1;
		"#,
		workspace_id as _,
	)
	.fetch_all(&mut *connection)
	.await
}