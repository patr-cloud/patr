use sqlx::{types::Uuid, Row};

use crate::{
	migrate_query as query,
	utils::{settings::Settings, Error},
	Database, service,
};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	migrate_dollars_to_cents(connection, config).await?;
	delete_region_permission(&mut *connection, config).await?;
	deleted_region_column(&mut *connection, config).await?;
	migrate_to_kubeconfig(&mut *connection, config).await?;
	byoc_for_digital_ocean(&mut *connection, config).await?;
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

pub(super) async fn migrate_dollars_to_cents(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	// transaction table migrations
	query!(
		r#"
		ALTER TABLE transaction
		RENAME COLUMN amount
		TO amount_in_cents;
		"#
	)
	.execute(&mut *connection)
	.await?;
	query!(
		r#"
		ALTER TABLE transaction
			ALTER COLUMN amount_in_cents TYPE BIGINT
				USING ROUND(amount_in_cents * 100)::BIGINT,
			ADD CONSTRAINT transaction_chk_amount_in_cents_positive
				CHECK(amount_in_cents >= 0);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// workspace table migrations
	query!(
		r#"
		ALTER TABLE workspace
		RENAME COLUMN amount_due
		TO amount_due_in_cents;
		"#
	)
	.execute(&mut *connection)
	.await?;
	query!(
		r#"
		ALTER TABLE workspace
			ALTER COLUMN amount_due_in_cents TYPE BIGINT
				USING ROUND(amount_due_in_cents * 100)::BIGINT,
			ADD CONSTRAINT workspace_chk_amount_due_in_cents_positive
				CHECK(amount_due_in_cents >= 0);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// coupon_code table migrations
	query!(
		r#"
		ALTER TABLE coupon_code
		RENAME COLUMN credits
		TO credits_in_cents;
		"#
	)
	.execute(&mut *connection)
	.await?;
	query!(
		r#"
		ALTER TABLE coupon_code
		ALTER COLUMN credits_in_cents TYPE BIGINT
		USING ROUND(credits_in_cents * 100)::BIGINT;
		"#
	)
	.execute(&mut *connection)
	.await?;
	query!(
		r#"
		ALTER TABLE coupon_code
		RENAME CONSTRAINT coupon_code_chk_credits_positive
		TO coupon_code_chk_credits_in_cents_positive;
		"#
	)
	.execute(&mut *connection)
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
		ADD COLUMN config_file TEXT,
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

async fn byoc_for_digital_ocean(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE deployment_region
		ADD COLUMN api_token TEXT,
		ADD COLUMN cluster_name TEXT,
		ADD COLUMN num_node INTEGER,
		ADD COLUMN node_name TEXT,
		ADD COLUMN node_size TEXT,
		ADD COLUMN do_cluster_id UUID;
		"#
	)
	.execute(&mut *connection)
	.await?;
	Ok(())
}
