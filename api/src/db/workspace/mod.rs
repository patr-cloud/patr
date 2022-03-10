use api_models::utils::Uuid;
use chrono::{DateTime, Utc};

use crate::{
	models::db_mapping::{Workspace, WorkspaceAuditLog},
	query,
	query_as,
	Database,
};

mod docker_registry;
mod domain;
mod infrastructure;
mod metrics;
mod secret;

pub use self::{docker_registry::*, domain::*, infrastructure::*, metrics::*};

pub async fn initialize_workspaces_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing workspace tables");
	query!(
		r#"
		CREATE TABLE workspace(
			id UUID
				CONSTRAINT workspace_pk PRIMARY KEY,
			name CITEXT NOT NULL
				CONSTRAINT workspace_uq_name UNIQUE,
			super_admin_id UUID NOT NULL
				CONSTRAINT workspace_fk_super_admin_id
					REFERENCES "user"(id),
			active BOOLEAN NOT NULL DEFAULT FALSE
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

	domain::initialize_domain_pre(connection).await?;
	docker_registry::initialize_docker_registry_pre(connection).await?;
	secret::initialize_secret_pre(connection).await?;
	infrastructure::initialize_infrastructure_pre(connection).await?;

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

	domain::initialize_domain_post(connection).await?;
	docker_registry::initialize_docker_registry_post(connection).await?;
	secret::initialize_secret_post(connection).await?;
	infrastructure::initialize_infrastructure_post(connection).await?;

	Ok(())
}

pub async fn create_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	name: &str,
	super_admin_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			workspace
		VALUES
			($1, $2, $3, $4);
		"#,
		workspace_id as _,
		name as _,
		super_admin_id as _,
		true,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn get_workspace_info(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<Option<Workspace>, sqlx::Error> {
	query_as!(
		Workspace,
		r#"
		SELECT
			id as "id: _",
			name::TEXT as "name!: _",
			super_admin_id as "super_admin_id: _",
			active
		FROM
			workspace
		WHERE
			id = $1;
		"#,
		workspace_id as _,
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_workspace_by_name(
	connection: &mut <Database as sqlx::Database>::Connection,
	name: &str,
) -> Result<Option<Workspace>, sqlx::Error> {
	query_as!(
		Workspace,
		r#"
		SELECT
			id as "id: _",
			name::TEXT as "name!: _",
			super_admin_id as "super_admin_id: _",
			active
		FROM
			workspace
		WHERE
			name = $1;
		"#,
		name as _
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn update_workspace_name(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	name: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			workspace
		SET
			name = $1
		WHERE
			id = $2;
		"#,
		name as _,
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
	date: DateTime<Utc>,
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
			workspace_audit_log
		VALUES
			($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12);
		"#,
		id as _,
		date,
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
