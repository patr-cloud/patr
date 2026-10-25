//! Adds the `actor` registry: one system-wide unique id per accountable
//! identity (a user or a service account), shared with its subtype row.
//!
//! `WORKSPACE_ACTOR_TYPE` becomes `ACTOR_TYPE`, since it now types the actor
//! itself and not just its workspace-scoped form. Every `actor_client` names
//! the actor that owns it, and the subtype tables' composite keys pin each
//! credential to its owner: a `user_login` to its user, a service account to
//! itself. Provenance (`role_binding.created_by`, `workspace_user_invite`'s
//! `invited_by`) points at `actor` instead of `"user"`, so it can name any
//! kind of actor and outlives memberships.
//!
//! Every existing user and service account (deleted ones too) gets its actor
//! row, and every `actor_client` is backfilled from its `user_login` or its
//! service account before `actor_id` becomes `NOT NULL`. A web login deleted
//! before this ran left its `actor_client` with no owner to backfill from:
//! those are dropped unless an audit row references them, in which case the
//! migration fails and needs a hand.

use crate::prelude::*;

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		ALTER TYPE WORKSPACE_ACTOR_TYPE RENAME TO ACTOR_TYPE;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE TABLE actor(
			id UUID NOT NULL,
			actor_type ACTOR_TYPE NOT NULL,
			created TIMESTAMPTZ NOT NULL
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		INSERT INTO
			actor
		SELECT
			id,
			'user',
			created
		FROM
			"user";
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		INSERT INTO
			actor
		SELECT
			id,
			'service_account',
			created
		FROM
			service_account;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE actor
			ADD CONSTRAINT actor_pk PRIMARY KEY(id),
			ADD CONSTRAINT actor_uq_id_actor_type UNIQUE(id, actor_type);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE "user"
			ADD COLUMN actor_type ACTOR_TYPE NOT NULL
				GENERATED ALWAYS AS ('user') STORED,
			ADD CONSTRAINT user_fk_id_actor_type
				FOREIGN KEY(id, actor_type) REFERENCES actor(id, actor_type);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE service_account
			ADD CONSTRAINT service_account_fk_id_actor_type_actor
				FOREIGN KEY(id, actor_type) REFERENCES actor(id, actor_type);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE actor_client
		ADD COLUMN actor_id UUID;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		UPDATE
			actor_client
		SET
			actor_id = user_login.user_id
		FROM
			user_login
		WHERE
			user_login.login_id = actor_client.id;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		UPDATE
			actor_client
		SET
			actor_id = id
		WHERE
			actor_client_type = 'service_account';
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DELETE FROM
			actor_client
		WHERE
			actor_id IS NULL AND
			NOT EXISTS(
				SELECT
					1
				FROM
					audit_log
				WHERE
					audit_log.actor_client_id = actor_client.id
			);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE actor_client
			ALTER COLUMN actor_id SET NOT NULL,
			ADD CONSTRAINT actor_client_fk_actor_id
				FOREIGN KEY(actor_id) REFERENCES actor(id),
			ADD CONSTRAINT actor_client_uq_id_actor_client_type_actor_id
				UNIQUE(id, actor_client_type, actor_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE user_login
			DROP CONSTRAINT user_login_fk_login_id_actor_client_type,
			ADD CONSTRAINT user_login_fk_login_id_actor_client_type_user_id
				FOREIGN KEY(login_id, actor_client_type, user_id)
					REFERENCES actor_client(id, actor_client_type, actor_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE service_account
			DROP CONSTRAINT service_account_fk_id_actor_client_type,
			ADD CONSTRAINT service_account_fk_id_actor_client_type_id
				FOREIGN KEY(id, actor_client_type, id)
					REFERENCES actor_client(id, actor_client_type, actor_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE actor_client
		DROP CONSTRAINT actor_client_uq_id_actor_client_type;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE role_binding
			DROP CONSTRAINT role_binding_fk_created_by,
			ADD CONSTRAINT role_binding_fk_created_by
				FOREIGN KEY(created_by) REFERENCES actor(id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE workspace_user_invite
			DROP CONSTRAINT workspace_user_invite_fk_invited_by,
			ADD CONSTRAINT workspace_user_invite_fk_invited_by
				FOREIGN KEY(invited_by) REFERENCES actor(id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
