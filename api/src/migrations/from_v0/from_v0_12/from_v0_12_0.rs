use sqlx::{types::Uuid, Row};

use crate::{
	migrate_query as query,
	service,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	delete_region_permission(&mut *connection, config).await?;
	deleted_region_column(&mut *connection, config).await?;
	migrate_to_kubeconfig(&mut *connection, config).await?;
	Ok(())
}

async fn delete_region_permission(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	let permission = "workspace::region::delete";
	let uuid = loop {
		let uuid = Uuid::new_v4();

		let exists = query!(
			r#"
			SELECT
				*
			FROM
				permission
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
			permission
		VALUES
			($1, $2, '');
		"#,
		&uuid,
		permission
	)
	.fetch_optional(&mut *connection)
	.await?;

	Ok(())
}

async fn deleted_region_column(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		CREATE TYPE REGION_STATUS AS ENUM(
			'created',
			'active',
			'deleted'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment_region
		ADD COLUMN config_file JSON,
		ADD COLUMN deleted TIMESTAMPTZ,
		ADD COLUMN status REGION_STATUS NOT NULL DEFAULT 'created';
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn migrate_to_kubeconfig(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	struct Region {
		pub id: Uuid,
		pub kubernetes_cluster_url: String,
		pub kubernetes_auth_username: String,
		pub kubernetes_auth_token: String,
		pub kubernetes_ca_data: String,
	}

	let regions = query!(
		r#"
		SELECT
			*
		FROM
			deployment_region;
		WHERE
			ready = true;
		"#
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| Region {
		id: row.get::<Uuid, _>("id"),
		kubernetes_cluster_url: row.get::<String, _>("kubernetes_cluster_url"),
		kubernetes_auth_username: row
			.get::<String, _>("kubernetes_auth_username"),
		kubernetes_auth_token: row.get::<String, _>("kubernetes_auth_token"),
		kubernetes_ca_data: row.get::<String, _>("kubernetes_ca_data"),
	});

	for region in regions {
		let kubeconfig = service::generate_kubeconfig_from_template(
			&region.kubernetes_cluster_url,
			&region.kubernetes_auth_username,
			&region.kubernetes_auth_token,
			&region.kubernetes_ca_data,
		);

		query!(
			r#"
			UPDATE
				deployment_region
			SET
				config_file = $1
			WHERE
				id = $2;
			"#,
			kubeconfig,
			region.id
		)
		.execute(&mut *connection)
		.await?;
	}

	query!(
		r#"
		ALTER TABLE deployment_region
		DROP CONSTRAINT deployment_region_chk_ready_or_not;
		
		"#,
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment_region
		DROP COLUMN kubernetes_cluster_url,
		DROP COLUME kubernetes_auth_username,
		DROP COLUME kubernetes_auth_token,
		DROP COLUME kubernetes_ca_data;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment_region
		ADD CONSTRAINT deployment_region_chk_ready_or_not CHECK(
			(
				workspace_id IS NOT NULL AND (
					(
						ready = TRUE AND
						config_file IS NOT NULL AND
						kubernetes_ingress_ip_addr IS NOT NULL
					) OR (
						ready = FALSE AND
						config_file IS NULL AND
						kubernetes_ingress_ip_addr IS NULL
					)
				)
			) OR (
				workspace_id IS NULL AND
				config_file IS NULL AND
				kubernetes_ingress_ip_addr IS NULL
			)
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
