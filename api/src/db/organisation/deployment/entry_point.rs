use sqlx::Transaction;

use crate::{
	models::db_mapping::{
		DeploymentEntryPoint,
		DeploymentEntryPointType,
		DeploymentEntryPointValue,
	},
	query,
	Database,
};

pub async fn initialize_entry_point_pre(
	transaction: &mut Transaction<'_, Database>,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing entry point tables");
	query!(
		r#"
		CREATE TYPE DEPLOYMENT_ENTRY_POINT_TYPE AS ENUM(
			'deployment',
			'redirect',
			'proxy'
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE deployment_entry_point(
			id BYTEA CONSTRAINT deployment_entry_point_pk PRIMARY KEY,
			sub_domain VARCHAR(255) NOT NULL DEFAULT '@',
			domain_id BYTEA NOT NULL
				CONSTRAINT deployment_entry_point_fk_domain_id
					REFERENCES organisation_domain(id),
			path VARCHAR(255) NOT NULL DEFAULT '/',
			entry_point_type DEPLOYMENT_ENTRY_POINT_TYPE NOT NULL,
			deployment_id BYTEA,
			deployment_port INTEGER
				CONSTRAINT deployment_entry_point_chk_deployment_port_u16
					CHECK(deployment_port >= 0 AND deployment_port <= 65534),
			url TEXT,
			CONSTRAINT deployment_entry_point_uq_sub_domain_domain_id_path
				UNIQUE(sub_domain, domain_id, path),
			CONSTRAINT deployment_entry_point_fk_deployment_id_deployment_port
				FOREIGN KEY(deployment_id, deployment_port)
				REFERENCES deployment_exposed_port(deployment_id, port),
			CONSTRAINT deployment_entry_point_chk_port_url_valid CHECK(
				(
					entry_point_type = 'deployment' AND
					(
						deployment_id IS NOT NULL AND
						deployment_port IS NOT NULL AND
						url IS NULL
					)
				) OR
				(
					(
						entry_point_type = 'redirect' OR
						entry_point_type = 'proxy'
					) AND
					(
						deployment_id IS NULL AND
						deployment_port IS NULL AND
						url IS NOT NULL
					)
				)
			)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	Ok(())
}

pub async fn initialize_entry_point_post(
	transaction: &mut Transaction<'_, Database>,
) -> Result<(), sqlx::Error> {
	log::info!("Finishing up entry point tables initialization");
	query!(
		r#"
		ALTER TABLE deployment_entry_point
		ADD CONSTRAINT deployment_entry_point_fk_id
		FOREIGN KEY(id) REFERENCES resource(id);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	Ok(())
}

pub async fn get_deployment_entry_points_in_organisation(
	connection: &mut Transaction<'_, Database>,
	organisation_id: &[u8],
) -> Result<Vec<DeploymentEntryPoint>, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT
			deployment_entry_point.id,
			deployment_entry_point.sub_domain,
			deployment_entry_point.domain_id,
			deployment_entry_point.path,
			deployment_entry_point.entry_point_type 
				as "entry_point_type: DeploymentEntryPointType",
			deployment_entry_point.deployment_id,
			deployment_entry_point.deployment_port,
			deployment_entry_point.url
		FROM
			deployment_entry_point
		INNER JOIN
			resource
		ON
			deployment_entry_point.id = resource.id
		WHERE
			resource.owner_id = $1;
		"#,
		organisation_id
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| DeploymentEntryPoint {
		id: row.id,
		sub_domain: if row.sub_domain == "@" {
			None
		} else {
			Some(row.sub_domain)
		},
		domain_id: row.domain_id,
		path: row.path,
		entry_point_type: match row.entry_point_type {
			DeploymentEntryPointType::Deployment => {
				DeploymentEntryPointValue::Deployment {
					deployment_id: row.deployment_id.unwrap(),
					deployment_port: row.deployment_port.unwrap() as u16,
				}
			}
			DeploymentEntryPointType::Redirect => {
				DeploymentEntryPointValue::Redirect {
					url: row.url.unwrap(),
				}
			}
			DeploymentEntryPointType::Proxy => {
				DeploymentEntryPointValue::Proxy {
					url: row.url.unwrap(),
				}
			}
		},
	})
	.collect();

	Ok(rows)
}

pub async fn get_deployment_entry_point_by_url(
	connection: &mut Transaction<'_, Database>,
	sub_domain: &str,
	domain_id: &[u8],
	path: &str,
) -> Result<Option<DeploymentEntryPoint>, sqlx::Error> {
	query!(
		r#"
		SELECT
			deployment_entry_point.id,
			deployment_entry_point.sub_domain,
			deployment_entry_point.domain_id,
			deployment_entry_point.path,
			deployment_entry_point.entry_point_type 
				as "entry_point_type: DeploymentEntryPointType",
			deployment_entry_point.deployment_id,
			deployment_entry_point.deployment_port,
			deployment_entry_point.url
		FROM
			deployment_entry_point
		WHERE
			sub_domain = $1 AND
			domain_id = $2 AND
			path = $3;
		"#,
		sub_domain,
		domain_id,
		path
	)
	.fetch_all(&mut *connection)
	.await
	.map(|rows| {
		rows.into_iter().next().map(|row| DeploymentEntryPoint {
			id: row.id,
			sub_domain: if row.sub_domain == "@" {
				None
			} else {
				Some(row.sub_domain)
			},
			domain_id: row.domain_id,
			path: row.path,
			entry_point_type: match row.entry_point_type {
				DeploymentEntryPointType::Deployment => {
					DeploymentEntryPointValue::Deployment {
						deployment_id: row.deployment_id.unwrap(),
						deployment_port: row.deployment_port.unwrap() as u16,
					}
				}
				DeploymentEntryPointType::Redirect => {
					DeploymentEntryPointValue::Redirect {
						url: row.url.unwrap(),
					}
				}
				DeploymentEntryPointType::Proxy => {
					DeploymentEntryPointValue::Proxy {
						url: row.url.unwrap(),
					}
				}
			},
		})
	})
}

pub async fn get_deployment_entry_point_by_id(
	connection: &mut Transaction<'_, Database>,
	entry_point_id: &[u8],
) -> Result<Option<DeploymentEntryPoint>, sqlx::Error> {
	query!(
		r#"
		SELECT
			id,
			sub_domain,
			domain_id,
			path,
			entry_point_type as "entry_point_type: DeploymentEntryPointType",
			deployment_id,
			deployment_port,
			url
		FROM
			deployment_entry_point
		WHERE
			id = $1;
		"#,
		entry_point_id
	)
	.fetch_all(&mut *connection)
	.await
	.map(|rows| {
		rows.into_iter().next().map(|row| DeploymentEntryPoint {
			id: row.id,
			sub_domain: if row.sub_domain == "@" {
				None
			} else {
				Some(row.sub_domain)
			},
			domain_id: row.domain_id,
			path: row.path,
			entry_point_type: match row.entry_point_type {
				DeploymentEntryPointType::Deployment => {
					DeploymentEntryPointValue::Deployment {
						deployment_id: row.deployment_id.unwrap(),
						deployment_port: row.deployment_port.unwrap() as u16,
					}
				}
				DeploymentEntryPointType::Redirect => {
					DeploymentEntryPointValue::Redirect {
						url: row.url.unwrap(),
					}
				}
				DeploymentEntryPointType::Proxy => {
					DeploymentEntryPointValue::Proxy {
						url: row.url.unwrap(),
					}
				}
			},
		})
	})
}

pub async fn add_deployment_entry_point_for_deployment(
	connection: &mut Transaction<'_, Database>,
	entry_point_id: &[u8],
	sub_domain: &str,
	domain_id: &[u8],
	path: &str,
	deployment_id: &[u8],
	deployment_port: u16,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			deployment_entry_point
		VALUES
			($1, $2, $3, $4, 'deployment', $5, $6, NULL);
		"#,
		entry_point_id,
		sub_domain,
		domain_id,
		path,
		deployment_id,
		deployment_port as i32
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn add_deployment_entry_point_for_redirect(
	connection: &mut Transaction<'_, Database>,
	entry_point_id: &[u8],
	sub_domain: &str,
	domain_id: &[u8],
	path: &str,
	url: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			deployment_entry_point
		VALUES
			($1, $2, $3, $4, 'redirect', NULL, NULL, $5);
		"#,
		entry_point_id,
		sub_domain,
		domain_id,
		path,
		url
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn add_deployment_entry_point_for_proxy(
	connection: &mut Transaction<'_, Database>,
	entry_point_id: &[u8],
	sub_domain: &str,
	domain_id: &[u8],
	path: &str,
	url: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			deployment_entry_point
		VALUES
			($1, $2, $3, $4, 'proxy', NULL, NULL, $5);
		"#,
		entry_point_id,
		sub_domain,
		domain_id,
		path,
		url
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_deployment_entry_point_to_deployment(
	connection: &mut Transaction<'_, Database>,
	entry_point_id: &[u8],
	deployment_id: &[u8],
	deployment_port: u16,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			deployment_entry_point
		SET
			entry_point_type = 'deployment',
			deployment_id = $1,
			deployment_port = $2,
			url = NULL
		WHERE
			id = $3;
		"#,
		deployment_id,
		deployment_port as i32,
		entry_point_id
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_deployment_entry_point_to_redirect(
	connection: &mut Transaction<'_, Database>,
	entry_point_id: &[u8],
	url: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			deployment_entry_point
		SET
			entry_point_type = 'redirect',
			deployment_id = NULL,
			deployment_port = NULL,
			url = $1
		WHERE
			id = $2;
		"#,
		url,
		entry_point_id
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_deployment_entry_point_to_proxy(
	connection: &mut Transaction<'_, Database>,
	entry_point_id: &[u8],
	url: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			deployment_entry_point
		SET
			entry_point_type = 'proxy',
			deployment_id = NULL,
			deployment_port = NULL,
			url = $1
		WHERE
			id = $2;
		"#,
		url,
		entry_point_id
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn delete_deployment_entry_point_by_id(
	connection: &mut Transaction<'_, Database>,
	entry_point_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			deployment_entry_point
		WHERE
			id = $1;
		"#,
		entry_point_id
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}
