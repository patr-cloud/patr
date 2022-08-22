use crate::{
	migrate_query as query,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	update_user_login_table_with_meta_info(&mut *connection, config).await?;
	Ok(())
}

async fn update_user_login_table_with_meta_info(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE workspace_audit_log
		DROP CONSTRAINT workspace_audit_log_fk_login_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DROP TABLE 
		user_login;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE user_login(
			login_id UUID CONSTRAINT user_login_uq_login_id UNIQUE,
			refresh_token TEXT NOT NULL,
			token_expiry TIMESTAMPTZ NOT NULL,
			user_id UUID NOT NULL
				CONSTRAINT user_login_fk_user_id REFERENCES "user"(id),
			last_login TIMESTAMPTZ NOT NULL,
			last_activity TIMESTAMPTZ NOT NULL,
			created TIMESTAMPTZ NOT NULL,
			created_ip INET NOT NULL,
			created_location GEOMETRY NOT NULL,
			last_activity_ip INET NOT NULL,
			last_activity_location GEOMETRY NOT NULL,
			last_activity_user_agent TEXT NOT NULL,
			CONSTRAINT user_login_pk PRIMARY KEY(login_id, user_id)
		);
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

	Ok(())
}
