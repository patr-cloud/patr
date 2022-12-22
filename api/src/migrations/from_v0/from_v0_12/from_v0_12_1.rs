use crate::{
	migrate_query as query,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	managed_url_redirects(&mut *connection, config).await?;
	Ok(())
}

pub(super) async fn managed_url_redirects(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE managed_url
		ADD COLUMN permanent_redirect BOOLEAN,
		ADD COLUMN http_only BOOLEAN;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			managed_url
		SET
			http_only = FALSE,
			permanent_redirect = FALSE
		WHERE
			url_type = 'redirect';
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			managed_url
		SET
			http_only = FALSE
		WHERE
			url_type = 'proxy_url';
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE managed_url
		DROP CONSTRAINT managed_url_chk_values_null_or_not_null;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE managed_url
		ADD CONSTRAINT managed_url_chk_values_null_or_not_null CHECK(
			(
				url_type = 'proxy_to_deployment' AND
				deployment_id IS NOT NULL AND
				port IS NOT NULL AND
				static_site_id IS NULL AND
				url IS NULL AND
				permanent_redirect IS NULL AND
				http_only IS NULL
			) OR
			(
				url_type = 'proxy_to_static_site' AND
				deployment_id IS NULL AND
				port IS NULL AND
				static_site_id IS NOT NULL AND
				url IS NULL AND
				permanent_redirect IS NULL AND
				http_only IS NULL
			) OR
			(
				url_type = 'proxy_url' AND
				deployment_id IS NULL AND
				port IS NULL AND
				static_site_id IS NULL AND
				url IS NOT NULL AND
				permanent_redirect IS NOT NULL AND
				http_only IS NOT NULL
			) OR
			(
				url_type = 'redirect' AND
				deployment_id IS NULL AND
				port IS NULL AND
				static_site_id IS NULL AND
				url IS NOT NULL AND
				permanent_redirect IS NOT NULL AND
				http_only IS NOT NULL
			)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
