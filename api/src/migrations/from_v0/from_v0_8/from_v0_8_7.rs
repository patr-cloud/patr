use crate::{
	migrate_query as query,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	add_last_unverified_column_to_workspace_domain(&mut *connection, config)
		.await?;
	create_user_transferring_domain_to_patr_table(&mut *connection, config)
		.await?;
	Ok(())
}
async fn add_last_unverified_column_to_workspace_domain(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE workspace_domain
		ADD COLUMN last_unverified TIMESTAMPTZ;
		"#
	)
	.execute(&mut *connection)
	.await?;
	Ok(())
}
async fn create_user_transferring_domain_to_patr_table(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		CREATE TABLE user_transferring_domain_to_patr(
			domain_id UUID NOT NULL
				CONSTRAINT user_transfer_domain_pk PRIMARY KEY,
			nameserver_type DOMAIN_NAMESERVER_TYPE NOT NULL
				CONSTRAINT user_transfer_domain_chk_nameserver_type CHECK(
					nameserver_type = 'external'
				),
			zone_identifier TEXT NOT NULL,
			is_verified BOOLEAN NOT NULL,
			CONSTRAINT user_transfer_domain_fk_domain_id_nameserver_type
				FOREIGN KEY(domain_id)REFERENCES
					user_controlled_domain(domain_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;
	query!(
		r#"
		ALTER TABLE workspace_domain
		ADD COLUMN transfer_domain UUID
		CONSTRAINT workspace_domain_fk_transfer_domain
		REFERENCES user_transferring_domain_to_patr(domain_id),
		ADD CONSTRAINT workspace_domain_chk_transfer_domain_ext CHECK(
			(
				transfer_domain IS NULL AND
				nameserver_type = 'internal'
			) OR
			(
				nameserver_type = 'external'
			)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;
	Ok(())
}
