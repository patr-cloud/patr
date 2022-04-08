use crate::{migrate_query as query, utils::settings::Settings, Database};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		SET CONSTRAINTS ALL DEFERRED;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			deployment_static_sites
		SET
			domain_name = CONCAT(
				'deleted.patr.cloud.',
				ENCODE(id, 'hex'),
				'.',
				REPLACE(
					domain_name,
					CONCAT(
						'deleted.patr.cloud.',
						ENCODE(id, 'hex')
					),
					''
				)
			)
		WHERE
			domain_name NOT LIKE CONCAT(
				'deleted.patr.cloud.',
				ENCODE(id, 'hex'),
				'.%'
			) AND
			status = 'deleted';
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			deployed_domain
		SET
			domain_name = deployment_static_sites.domain_name
		FROM
			deployment_static_sites
		WHERE
			deployed_domain.static_site_id = deployment_static_sites.id AND
			deployment_static_sites.status = 'deleted';
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		SET CONSTRAINTS ALL IMMEDIATE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
