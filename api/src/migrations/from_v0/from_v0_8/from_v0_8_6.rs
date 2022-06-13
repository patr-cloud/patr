use api_models::utils::Uuid;

use crate::{
	migrate_query as query,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	create_static_site_deploy_history(&mut *connection, config).await?;
	add_static_site_upload_resource_type(&mut *connection, config).await?;

	Ok(())
}

async fn create_static_site_deploy_history(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		CREATE TABLE static_site_deploy_history(
			upload_id UUID CONSTRAINT deployment_static_site_history_pk PRIMARY KEY,
			static_site_id UUID NOT NULL,
			message TEXT NOT NULL,
			created BIGINT NOT NULL
				CONSTRAINT static_site_deploy_history_chk_created_unsigned CHECK(
						created >= 0
				),
			CONSTRAINT static_site_deploy_history_fk_static_site_id
				FOREIGN KEY(static_site_id)
					REFERENCES deployment_static_site(id),
			CONSTRAINT static_site_deploy_history_uq_upload_id_static_site_id
				UNIQUE(upload_id, static_site_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;
	Ok(())
}

async fn add_static_site_upload_resource_type(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	let uuid = loop {
		let uuid = Uuid::new_v4();

		let exists = query!(
			r#"
			SELECT
				*
			FROM
				resource_type
			WHERE
				id = $1;
			"#,
			&uuid
		)
		.fetch_optional(&mut *connection)
		.await?
		.is_some();

		if !exists {
			break uuid;
		}
	};

	query!(
		r#"
		INSERT INTO
			resource_type(
				id,
				name,
				description
			)
		VALUES
			($1, 'staticSiteUpload', NULL);
		"#,
		&uuid,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
