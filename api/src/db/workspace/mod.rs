use api_models::utils::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{query, query_as, Database};

mod billing;
mod ci;
mod docker_registry;
mod domain;
mod infrastructure;
mod metrics;
mod region;
mod secret;

pub use self::{
	billing::*,
	ci::*,
	docker_registry::*,
	domain::*,
	infrastructure::*,
	metrics::*,
	region::*,
	secret::*,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
	pub id: Uuid,
	pub name: String,
	pub super_admin_id: Uuid,
	pub active: bool,
	pub alert_emails: Vec<String>,
	pub payment_type: PaymentType,
	pub default_payment_method_id: Option<String>,
	pub deployment_limit: i32,
	pub database_limit: i32,
	pub static_site_limit: i32,
	pub managed_url_limit: i32,
	pub docker_repository_storage_limit: i32,
	pub domain_limit: i32,
	pub secret_limit: i32,
	pub volume_storage_limit: i32,
	pub stripe_customer_id: String,
	pub address_id: Option<Uuid>,
	pub amount_due_in_cents: u64,
	pub is_spam: bool,
}

pub struct WorkspaceAuditLog {
	pub id: Uuid,
	pub date: DateTime<Utc>,
	pub ip_address: String,
	pub workspace_id: Uuid,
	pub user_id: Option<Uuid>,
	pub login_id: Option<Uuid>,
	pub resource_id: Uuid,
	pub action: String,
	pub request_id: Uuid,
	pub metadata: serde_json::Value,
	pub patr_action: bool,
	pub success: bool,
}

#[derive(sqlx::Type, Debug, Clone, Serialize, Deserialize)]
#[sqlx(type_name = "PAYMENT_TYPE", rename_all = "lowercase")]
pub enum PaymentType {
	Card,
	Enterprise,
}

#[derive(Debug, Clone)]
pub struct WorkspaceCredits {
	pub workspace_id: Uuid,
	pub credits: i64,
	pub metadata: serde_json::Value,
	pub date: DateTime<Utc>,
}

pub struct Address {
	pub id: Uuid,
	pub first_name: String,
	pub last_name: String,
	pub address_line_1: String,
	pub address_line_2: Option<String>,
	pub address_line_3: Option<String>,
	pub city: String,
	pub state: String,
	pub zip: String,
	pub country: String,
}

pub async fn initialize_workspaces_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing workspace tables");

	query!(
		r#"
		CREATE TYPE PAYMENT_TYPE AS ENUM(
			'card',
			'enterprise'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE workspace(
			id UUID
				CONSTRAINT workspace_pk PRIMARY KEY,
			name CITEXT NOT NULL,
			super_admin_id UUID NOT NULL
				CONSTRAINT workspace_fk_super_admin_id
					REFERENCES "user"(id),
			active BOOLEAN NOT NULL DEFAULT FALSE,
			alert_emails VARCHAR(320) [] NOT NULL,
			payment_type PAYMENT_TYPE NOT NULL,
			default_payment_method_id TEXT,
			deployment_limit INTEGER NOT NULL,
			database_limit INTEGER NOT NULL,
			static_site_limit INTEGER NOT NULL,
			managed_url_limit INTEGER NOT NULL,
			docker_repository_storage_limit INTEGER NOT NULL,
			domain_limit INTEGER NOT NULL,
			secret_limit INTEGER NOT NULL,
			stripe_customer_id TEXT NOT NULL,
			address_id UUID,
			amount_due_in_cents BIGINT NOT NULL
				CONSTRAINT workspace_chk_amount_due_in_cents_positive
					CHECK (amount_due_in_cents >= 0),
			deleted TIMESTAMPTZ,
			is_spam BOOLEAN NOT NULL,
			volume_storage_limit INTEGER NOT NULL,
			CONSTRAINT workspace_uq_id_super_admin_id
				UNIQUE(id, super_admin_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			workspace_idx_super_admin_id
		ON
			workspace
		(super_admin_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			workspace_idx_active
		ON
			workspace
		(active);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// Ref: https://www.postgresql.org/docs/13/datatype-enum.html
	query!(
		r#"
		CREATE TYPE RESOURCE_OWNER_TYPE AS ENUM(
			'personal',
			'business'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE workspace_audit_log (
			id UUID NOT NULL CONSTRAINT workspace_audit_log_pk PRIMARY KEY,
			date TIMESTAMPTZ NOT NULL,
			ip_address TEXT NOT NULL,
			workspace_id UUID NOT NULL
				CONSTRAINT workspace_audit_log_fk_workspace_id
					REFERENCES workspace(id),
			user_id UUID,
			login_id UUID,
			resource_id UUID NOT NULL,
			action UUID NOT NULL,
			request_id UUID NOT NULL,
			metadata JSON NOT NULL,
			patr_action BOOL NOT NULL,
			success BOOL NOT NULL,
			CONSTRAINT workspace_audit_log_chk_patr_action CHECK(
				(
					patr_action = true AND
					user_id IS NULL AND
					login_id IS NULL
				) OR
				(
					patr_action = false AND
					user_id IS NOT NULL AND
					login_id IS NOT NULL
				)
			)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE address(
			id UUID NOT NULL CONSTRAINT address_pk PRIMARY KEY,
			first_name TEXT NOT NULL,
			last_name TEXT NOT NULL,
			address_line_1 TEXT NOT NULL,
			address_line_2 TEXT,
			address_line_3 TEXT,
			city TEXT NOT NULL,
			state TEXT NOT NULL,
			zip TEXT NOT NULL,
			country TEXT NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	domain::initialize_domain_pre(connection).await?;
	docker_registry::initialize_docker_registry_pre(connection).await?;
	secret::initialize_secret_pre(connection).await?;
	region::initialize_region_pre(connection).await?;
	infrastructure::initialize_infrastructure_pre(connection).await?;
	billing::initialize_billing_pre(connection).await?;
	ci::initialize_ci_pre(connection).await?;

	Ok(())
}

pub async fn initialize_workspaces_post(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Finishing up workspace tables initialization");
	query!(
		r#"
		ALTER TABLE workspace
		ADD CONSTRAINT workspace_fk_id
		FOREIGN KEY(id) REFERENCES resource(id)
		DEFERRABLE INITIALLY IMMEDIATE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace_audit_log
		ADD CONSTRAINT workspace_audit_log_fk_user_id
		FOREIGN KEY(user_id) REFERENCES "user"(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace_audit_log
		ADD CONSTRAINT workspace_audit_log_fk_login_id
		FOREIGN KEY(user_id, login_id) REFERENCES user_login(user_id, login_id);
	"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace_audit_log
		ADD CONSTRAINT workspace_audit_log_fk_resource_id
		FOREIGN KEY(resource_id) REFERENCES resource(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace_audit_log
		ADD CONSTRAINT workspace_audit_log_fk_action
		FOREIGN KEY(action) REFERENCES permission(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace
		ADD CONSTRAINT workspace_default_payment_method_id_fk
		FOREIGN KEY (default_payment_method_id)
			REFERENCES payment_method(payment_method_id)
		DEFERRABLE INITIALLY DEFERRED;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace
		ADD CONSTRAINT workspace_fk_address_id
		FOREIGN KEY(address_id) REFERENCES address(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE UNIQUE INDEX
			workspace_uq_name
		ON
			workspace(name)
		WHERE
			deleted IS NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	domain::initialize_domain_post(connection).await?;
	docker_registry::initialize_docker_registry_post(connection).await?;
	secret::initialize_secret_post(connection).await?;
	region::initialize_region_post(connection).await?;
	infrastructure::initialize_infrastructure_post(connection).await?;
	billing::initialize_billing_post(connection).await?;
	ci::initialize_ci_post(connection).await?;

	Ok(())
}

pub async fn create_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	name: &str,
	super_admin_id: &Uuid,
	alert_emails: &[String],
	deployment_limit: i32,
	database_limit: i32,
	static_site_limit: i32,
	managed_url_limit: i32,
	docker_repository_storage_limit: i32,
	domain_limit: i32,
	secret_limit: i32,
	volume_storage_limit: i32,
	stripe_customer_id: &str,
	payment_type: &PaymentType,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			workspace(
				id,
				name,
				super_admin_id,
				active,
				alert_emails,
				payment_type,
				default_payment_method_id,
				deployment_limit,
				database_limit,
				static_site_limit,
				managed_url_limit,
				docker_repository_storage_limit,
				domain_limit,
				secret_limit,
				volume_storage_limit,
				stripe_customer_id,
				address_id,
				amount_due_in_cents,
				is_spam
			)
		VALUES
			(
				$1,
				$2,
				$3,
				$4,
				$5,
				$6,
				NULL,
				$7,
				$8,
				$9,
				$10,
				$11,
				$12,
				$13,
				$14,
				$15,
				NULL,
				$16,
				FALSE
			);
		"#,
		workspace_id as _,
		name as _,
		super_admin_id as _,
		true,
		alert_emails as _,
		payment_type as _,
		deployment_limit,
		database_limit,
		static_site_limit,
		managed_url_limit,
		docker_repository_storage_limit,
		domain_limit,
		secret_limit,
		volume_storage_limit,
		stripe_customer_id,
		0 as i32
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_workspace_info(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<Option<Workspace>, sqlx::Error> {
	query!(
		r#"
		SELECT
			id as "id: Uuid",
			name::TEXT as "name!: String",
			super_admin_id as "super_admin_id: Uuid",
			active,
			alert_emails as "alert_emails: Vec<String>",
			payment_type as "payment_type: PaymentType",
			default_payment_method_id as "default_payment_method_id: String",
			deployment_limit,
			static_site_limit,
			database_limit,
			managed_url_limit,
			secret_limit,
			domain_limit,
			docker_repository_storage_limit,
			volume_storage_limit,
			stripe_customer_id,
			address_id as "address_id: Uuid",
			amount_due_in_cents,
			is_spam
		FROM
			workspace
		WHERE
			id = $1 AND
			deleted IS NULL;
		"#,
		workspace_id as _,
	)
	.fetch_optional(&mut *connection)
	.await
	.map(|row| {
		row.map(|row| Workspace {
			id: row.id,
			name: row.name,
			super_admin_id: row.super_admin_id,
			active: row.active,
			alert_emails: row.alert_emails,
			payment_type: row.payment_type,
			default_payment_method_id: row.default_payment_method_id,
			deployment_limit: row.deployment_limit,
			static_site_limit: row.static_site_limit,
			database_limit: row.database_limit,
			managed_url_limit: row.managed_url_limit,
			secret_limit: row.secret_limit,
			domain_limit: row.domain_limit,
			docker_repository_storage_limit: row
				.docker_repository_storage_limit,
			volume_storage_limit: row.volume_storage_limit,
			stripe_customer_id: row.stripe_customer_id,
			address_id: row.address_id,
			amount_due_in_cents: row.amount_due_in_cents as u64,
			is_spam: row.is_spam,
		})
	})
}

pub async fn get_workspace_by_name(
	connection: &mut <Database as sqlx::Database>::Connection,
	name: &str,
) -> Result<Option<Workspace>, sqlx::Error> {
	query!(
		r#"
		SELECT
			id as "id: Uuid",
			name::TEXT as "name!: String",
			super_admin_id as "super_admin_id: Uuid",
			active,
			alert_emails as "alert_emails: Vec<String>",
			payment_type as "payment_type: PaymentType",
			default_payment_method_id as "default_payment_method_id: String",
			deployment_limit,
			static_site_limit,
			database_limit,
			managed_url_limit,
			secret_limit,
			domain_limit,
			docker_repository_storage_limit,
			volume_storage_limit,
			stripe_customer_id,
			address_id as "address_id: Uuid",
			amount_due_in_cents,
			is_spam
		FROM
			workspace
		WHERE
			name = $1;
		"#,
		name as _
	)
	.fetch_optional(&mut *connection)
	.await
	.map(|row| {
		row.map(|row| Workspace {
			id: row.id,
			name: row.name,
			super_admin_id: row.super_admin_id,
			active: row.active,
			alert_emails: row.alert_emails,
			payment_type: row.payment_type,
			default_payment_method_id: row.default_payment_method_id,
			deployment_limit: row.deployment_limit,
			static_site_limit: row.static_site_limit,
			database_limit: row.database_limit,
			managed_url_limit: row.managed_url_limit,
			secret_limit: row.secret_limit,
			domain_limit: row.domain_limit,
			docker_repository_storage_limit: row
				.docker_repository_storage_limit,
			volume_storage_limit: row.volume_storage_limit,
			stripe_customer_id: row.stripe_customer_id,
			address_id: row.address_id,
			amount_due_in_cents: row.amount_due_in_cents as u64,
			is_spam: row.is_spam,
		})
	})
}

pub async fn delete_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	deletion_time: &DateTime<Utc>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			workspace
		SET
			deleted = $2
		WHERE
			id = $1;
		"#,
		workspace_id as _,
		deletion_time
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_workspace_info(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	name: Option<String>,
	alert_emails: Option<Vec<String>>,
	default_payment_method_id: Option<String>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			workspace
		SET
			name = COALESCE($1, name),
			alert_emails = COALESCE($2, alert_emails),
			default_payment_method_id = COALESCE($3, default_payment_method_id)
		WHERE
			id = $4;
		"#,
		name as _,
		alert_emails as _,
		default_payment_method_id as _,
		workspace_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn create_workspace_audit_log(
	connection: &mut <Database as sqlx::Database>::Connection,
	id: &Uuid,
	workspace_id: &Uuid,
	ip_address: &str,
	date: &DateTime<Utc>,
	user_id: Option<&Uuid>,
	login_id: Option<&Uuid>,
	resource_id: &Uuid,
	action: &Uuid,
	request_id: &Uuid,
	metadata: &serde_json::Value,
	patr_action: bool,
	success: bool,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			workspace_audit_log(
				id,
				date,
				ip_address,
				workspace_id,
				user_id,
				login_id,
				resource_id,
				action,
				request_id,
				metadata,
				patr_action,
				success
			)
		VALUES
			($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12);
		"#,
		id as _,
		date as _,
		ip_address,
		workspace_id as _,
		user_id as _,
		login_id as _,
		resource_id as _,
		action as _,
		request_id as _,
		metadata,
		patr_action,
		success,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn generate_new_workspace_audit_log_id(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Uuid, sqlx::Error> {
	loop {
		let uuid = Uuid::new_v4();

		let exists = query!(
			r#"
			SELECT
				*
			FROM
				workspace_audit_log
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

pub async fn get_workspace_audit_logs(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<Vec<WorkspaceAuditLog>, sqlx::Error> {
	query_as!(
		WorkspaceAuditLog,
		r#"
		SELECT
			workspace_audit_log.id as "id: _",
			date as "date: _",
			ip_address,
			workspace_id as "workspace_id: _",
			user_id as "user_id: _",
			login_id as "login_id: _",
			resource_id as "resource_id: _",
			permission.name as "action",
			request_id as "request_id: _",
			metadata as "metadata: _",
			patr_action as "patr_action: _",
			success
		FROM
			workspace_audit_log
		INNER JOIN
			permission
		ON
			permission.id = workspace_audit_log.action
		WHERE
			workspace_id = $1;
		"#,
		workspace_id as _
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn get_resource_audit_logs(
	connection: &mut <Database as sqlx::Database>::Connection,
	resource_id: &Uuid,
) -> Result<Vec<WorkspaceAuditLog>, sqlx::Error> {
	query_as!(
		WorkspaceAuditLog,
		r#"
		SELECT
			workspace_audit_log.id as "id: _",
			date as "date: _",
			ip_address,
			workspace_id as "workspace_id: _",
			user_id as "user_id: _",
			login_id as "login_id: _",
			resource_id as "resource_id: _",
			permission.name as "action",
			request_id as "request_id: _",
			metadata as "metadata: _",
			patr_action as "patr_action: _",
			success
		FROM
			workspace_audit_log
		INNER JOIN
			permission
		ON
			permission.id = workspace_audit_log.action
		WHERE
			resource_id = $1;
		"#,
		resource_id as _
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn get_all_workspaces(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Vec<Workspace>, sqlx::Error> {
	let res = query!(
		r#"
		SELECT DISTINCT
			id as "id: Uuid",
			name::TEXT as "name!: String",
			super_admin_id as "super_admin_id: Uuid",
			active,
			alert_emails as "alert_emails: Vec<String>",
			payment_type as "payment_type: PaymentType",
			default_payment_method_id as "default_payment_method_id: String",
			deployment_limit,
			static_site_limit,
			database_limit,
			managed_url_limit,
			secret_limit,
			domain_limit,
			docker_repository_storage_limit,
			volume_storage_limit,
			stripe_customer_id,
			address_id as "address_id: Uuid",
			amount_due_in_cents,
			is_spam
		FROM
			workspace
		WHERE
			deleted IS NULL;
		"#,
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| Workspace {
		id: row.id,
		name: row.name,
		super_admin_id: row.super_admin_id,
		active: row.active,
		alert_emails: row.alert_emails,
		payment_type: row.payment_type,
		default_payment_method_id: row.default_payment_method_id,
		deployment_limit: row.deployment_limit,
		static_site_limit: row.static_site_limit,
		database_limit: row.database_limit,
		managed_url_limit: row.managed_url_limit,
		secret_limit: row.secret_limit,
		domain_limit: row.domain_limit,
		docker_repository_storage_limit: row.docker_repository_storage_limit,
		volume_storage_limit: row.volume_storage_limit,
		stripe_customer_id: row.stripe_customer_id,
		address_id: row.address_id,
		amount_due_in_cents: row.amount_due_in_cents as u64,
		is_spam: row.is_spam,
	})
	.collect();

	Ok(res)
}

pub async fn get_resource_limit_for_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<u32, sqlx::Error> {
	query!(
		r#"
		SELECT (
				deployment_limit +
				database_limit +
				static_site_limit +
				managed_url_limit +
				domain_limit +
				secret_limit
			) as "limit!: i32"
		FROM
			workspace
		WHERE
			workspace.id = $1;
		"#,
		workspace_id as _,
	)
	.fetch_one(&mut *connection)
	.await
	.map(|row| row.limit as u32)
}

pub async fn generate_new_address_id(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Uuid, sqlx::Error> {
	loop {
		let uuid = Uuid::new_v4();
		let exists = query!(
			r#"
			SELECT
				*
			FROM
				address
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

pub async fn add_billing_address(
	connection: &mut <Database as sqlx::Database>::Connection,
	address_details: &Address,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			address(
				id,
				first_name,
				last_name,
				address_line_1,
				address_line_2,
				address_line_3,
				city,
				state,
				zip,
				country
			)
		VALUES (
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
		address_details.id as _,
		address_details.first_name,
		address_details.last_name,
		address_details.address_line_1,
		address_details.address_line_2,
		address_details.address_line_3,
		address_details.city,
		address_details.state,
		address_details.zip,
		address_details.country,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_billing_address(
	connection: &mut <Database as sqlx::Database>::Connection,
	address_details: &Address,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			address
		SET
			first_name = $2,
			last_name = $3,
			address_line_1 = $4,
			address_line_2 = $5,
			address_line_3 = $6,
			city = $7,
			state = $8,
			zip = $9,
			country = $10
		WHERE
			id = $1;
		"#,
		address_details.id as _,
		address_details.first_name,
		address_details.last_name,
		address_details.address_line_1,
		address_details.address_line_2,
		address_details.address_line_3,
		address_details.city,
		address_details.state,
		address_details.zip,
		address_details.country,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn add_billing_address_to_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	address_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			workspace
		SET
			address_id = $1
		WHERE
			id = $2;
			"#,
		address_id as _,
		workspace_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn delete_billing_address_from_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			workspace
		SET
			address_id = NULL
		WHERE
			id = $1;
		"#,
		workspace_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn delete_billing_address(
	connection: &mut <Database as sqlx::Database>::Connection,
	address_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			address
		WHERE
			id = $1;
		"#,
		address_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_billing_address(
	connection: &mut <Database as sqlx::Database>::Connection,
	address_id: &Uuid,
) -> Result<Option<Address>, sqlx::Error> {
	query_as!(
		Address,
		r#"
		SELECT
			id as "id: _",
			first_name,
			last_name,
			address_line_1,
			address_line_2,
			address_line_3,
			city,
			state,
			zip,
			country
		FROM
			address
		WHERE
			id = $1;
		"#,
		address_id as _,
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn set_default_payment_method_for_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	payment_method_id: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			workspace
		SET
			default_payment_method_id = $1
		WHERE
			id = $2
		"#,
		payment_method_id,
		workspace_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_default_payment_method_for_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<Option<String>, sqlx::Error> {
	query!(
		r#"
		SELECT
			default_payment_method_id
		FROM
			workspace
		WHERE
			id = $1
		"#,
		workspace_id as _,
	)
	.fetch_optional(&mut *connection)
	.await
	.map(|optional_record| {
		optional_record.and_then(|record| record.default_payment_method_id)
	})
}

pub async fn set_resource_limit_for_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	deployment_limit: i32,
	database_limit: i32,
	static_site_limit: i32,
	managed_url_limit: i32,
	docker_repository_storage_limit: i32,
	domain_limit: i32,
	secret_limit: i32,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			workspace
		SET
			deployment_limit = $1,
			database_limit = $2,
			static_site_limit = $3,
			managed_url_limit = $4,
			docker_repository_storage_limit = $5,
			domain_limit = $6,
			secret_limit = $7
		WHERE
			id = $8;
		"#,
		deployment_limit,
		database_limit,
		static_site_limit,
		managed_url_limit,
		docker_repository_storage_limit,
		domain_limit,
		secret_limit,
		workspace_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn mark_workspace_as_spam(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			workspace
		SET
			is_spam = TRUE
		WHERE
			id = $1;
		"#,
		workspace_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}
