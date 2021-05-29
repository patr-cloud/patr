use sqlx::{MySql, Transaction};

use crate::{
	models::db_mapping::{DeploymentEntryPoint, DeploymentEntryPointType},
	query,
};

pub async fn initialize_entry_point_pre(
	transaction: &mut Transaction<'_, MySql>,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing entry point tables");
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS deployment_entry_point(
			id BINARY(16) PRIMARY KEY,
			sub_domain VARCHAR(255) NOT NULL DEFAULT "@",
			domain_id BINARY(16) NOT NULL,
			path VARCHAR(255) NOT NULL DEFAULT "/",
			entry_point_type ENUM('deployment', 'redirect', 'proxy') NOT NULL,
			deployment_id BINARY(16),
			deployment_port SMALLINT UNSIGNED,
			url TEXT,
			UNIQUE (sub_domain, domain_id, path),
			FOREIGN KEY(domain_id) REFERENCES organisation_domain(id),
			FOREIGN KEY(deployment_id, deployment_port)
			REFERENCES deployment_exposed_port(deployment_id, port),
			CONSTRAINT CHECK(
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
	transaction: &mut Transaction<'_, MySql>,
) -> Result<(), sqlx::Error> {
	log::info!("Finishing up entry point tables initialization");
	query!(
		r#"
		ALTER TABLE deployment_entry_point
		ADD CONSTRAINT
		FOREIGN KEY (id) REFERENCES resource(id);
		"#
	)
	.execute(transaction)
	.await?;

	Ok(())
}

pub async fn get_deployment_entry_points_in_organisation(
	connection: &mut Transaction<'_, MySql>,
	organisation_id: &[u8],
) -> Result<Vec<DeploymentEntryPoint>, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT
			deployment_entry_point.*
		FROM
			deployment_entry_point
		INNER JOIN
			resource
		ON
			deployment_entry_point.id = resource.id
		WHERE
			resource.owner_id = ?;
		"#,
		organisation_id
	)
	.fetch_all(connection)
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
		entry_point_type: match row.entry_point_type.as_str() {
			"deployment" => DeploymentEntryPointType::Deployment {
				deployment_id: row.deployment_id.unwrap(),
				deployment_port: row.deployment_port.unwrap(),
			},
			"redirect" => DeploymentEntryPointType::Redirect {
				url: row.url.unwrap(),
			},
			"proxy" => DeploymentEntryPointType::Proxy {
				url: row.url.unwrap(),
			},
			_ => panic!(
				"{} {}",
				"Database shouldn't allow any other entry point type.",
				"How did this happen?"
			),
		},
	})
	.collect();

	Ok(rows)
}

pub async fn get_deployment_entry_point_by_url(
	connection: &mut Transaction<'_, MySql>,
	sub_domain: &str,
	domain_id: &[u8],
	path: &str,
) -> Result<Option<DeploymentEntryPoint>, sqlx::Error> {
	query!(
		r#"
		SELECT
			*
		FROM
			deployment_entry_point
		WHERE
			sub_domain = ? AND
			domain_id = ? AND
			path = ?;
		"#,
		sub_domain,
		domain_id,
		path
	)
	.fetch_all(connection)
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
			entry_point_type: match row.entry_point_type.as_str() {
				"deployment" => DeploymentEntryPointType::Deployment {
					deployment_id: row.deployment_id.unwrap(),
					deployment_port: row.deployment_port.unwrap(),
				},
				"redirect" => DeploymentEntryPointType::Redirect {
					url: row.url.unwrap(),
				},
				"proxy" => DeploymentEntryPointType::Proxy {
					url: row.url.unwrap(),
				},
				_ => panic!(
					"{} {}",
					"Database shouldn't allow any other entry point type.",
					"How did this happen?"
				),
			},
		})
	})
}

pub async fn get_deployment_entry_point_by_id(
	connection: &mut Transaction<'_, MySql>,
	entry_point_id: &[u8],
) -> Result<Option<DeploymentEntryPoint>, sqlx::Error> {
	query!(
		r#"
		SELECT
			*
		FROM
			deployment_entry_point
		WHERE
			id = ?;
		"#,
		entry_point_id
	)
	.fetch_all(connection)
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
			entry_point_type: match row.entry_point_type.as_str() {
				"deployment" => DeploymentEntryPointType::Deployment {
					deployment_id: row.deployment_id.unwrap(),
					deployment_port: row.deployment_port.unwrap(),
				},
				"redirect" => DeploymentEntryPointType::Redirect {
					url: row.url.unwrap(),
				},
				"proxy" => DeploymentEntryPointType::Proxy {
					url: row.url.unwrap(),
				},
				_ => panic!(
					"{} {}",
					"Database shouldn't allow any other entry point type.",
					"How did this happen?"
				),
			},
		})
	})
}

pub async fn add_deployment_entry_point_for_deployment(
	connection: &mut Transaction<'_, MySql>,
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
			(?, ?, ?, ?, 'deployment', ?, ?, NULL);
		"#,
		entry_point_id,
		sub_domain,
		domain_id,
		path,
		deployment_id,
		deployment_port
	)
	.execute(connection)
	.await
	.map(|_| ())
}

pub async fn add_deployment_entry_point_for_redirect(
	connection: &mut Transaction<'_, MySql>,
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
			(?, ?, ?, ?, 'redirect', NULL, NULL, ?);
		"#,
		entry_point_id,
		sub_domain,
		domain_id,
		path,
		url
	)
	.execute(connection)
	.await
	.map(|_| ())
}

pub async fn add_deployment_entry_point_for_proxy(
	connection: &mut Transaction<'_, MySql>,
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
			(?, ?, ?, ?, 'proxy', NULL, NULL, ?);
		"#,
		entry_point_id,
		sub_domain,
		domain_id,
		path,
		url
	)
	.execute(connection)
	.await
	.map(|_| ())
}

pub async fn update_deployment_entry_point_to_deployment(
	connection: &mut Transaction<'_, MySql>,
	entry_point_id: &[u8],
	deployment_id: &[u8],
	deployment_port: u16,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			deployment_entry_point
		SET
			entry_point_type = 'deployment' AND
			deployment_id = ? AND
			deployment_port = ? AND
			url = NULL
		WHERE
			id = ?;
		"#,
		deployment_id,
		deployment_port,
		entry_point_id
	)
	.execute(connection)
	.await
	.map(|_| ())
}

pub async fn update_deployment_entry_point_to_redirect(
	connection: &mut Transaction<'_, MySql>,
	entry_point_id: &[u8],
	url: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			deployment_entry_point
		SET
			entry_point_type = 'redirect' AND
			deployment_id = NULL AND
			deployment_port = NULL AND
			url = ?
		WHERE
			id = ?;
		"#,
		url,
		entry_point_id
	)
	.execute(connection)
	.await
	.map(|_| ())
}

pub async fn update_deployment_entry_point_to_proxy(
	connection: &mut Transaction<'_, MySql>,
	entry_point_id: &[u8],
	url: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			deployment_entry_point
		SET
			entry_point_type = 'proxy' AND
			deployment_id = NULL AND
			deployment_port = NULL AND
			url = ?
		WHERE
			id = ?;
		"#,
		url,
		entry_point_id
	)
	.execute(connection)
	.await
	.map(|_| ())
}

pub async fn delete_deployment_entry_point_by_id(
	connection: &mut Transaction<'_, MySql>,
	entry_point_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			deployment_entry_point
		WHERE
			id = ?
		"#,
		entry_point_id
	)
	.execute(connection)
	.await
	.map(|_| ())
}
