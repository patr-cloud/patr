use crate::{
	models::db_mapping::{
		AvgDistance,
		Deployment,
		DeploymentMachineType,
		DeploymentStatus,
		Method,
		Protocol,
	},
	query,
	query_as,
	Database,
};

pub async fn initialize_deployment_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing deployments tables");

	query!(
		r#"
		CREATE EXTENSION IF NOT EXISTS postgis;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE DEPLOYMENT_STATUS AS ENUM(
			'created', /* Created, but nothing pushed to it yet */
			'pushed', /* Something is pushed, but the system has not deployed it yet */
			'deploying', /* Something is pushed, and the system is currently deploying it */
			'running', /* Deployment is running successfully */
			'stopped', /* Deployment is stopped by the user */
			'errored', /* Deployment is stopped because of too many errors */
			'deleted' /* Deployment is deleted by the user */
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE DEPLOYMENT_MACHINE_TYPE AS ENUM(
			'micro',
			'small',
			'medium',
			'large'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE PROTOCOL AS ENUM(
			'http',
			'https'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE METHOD AS ENUM(
			'post',
			'get',
			'put',
			'patch',
			'delete'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE deployment(
			id BYTEA CONSTRAINT deployment_pk PRIMARY KEY,
			name VARCHAR(255) NOT NULL,
			registry VARCHAR(255) NOT NULL DEFAULT 'registry.patr.cloud',
			repository_id BYTEA CONSTRAINT deployment_fk_repository_id
				REFERENCES docker_registry_repository(id),
			image_name VARCHAR(512),
			image_tag VARCHAR(255) NOT NULL,
			status DEPLOYMENT_STATUS NOT NULL DEFAULT 'created',
			deployed_image TEXT,
			digitalocean_app_id TEXT
				CONSTRAINT deployment_uq_digitalocean_app_id UNIQUE,
			region TEXT NOT NULL DEFAULT 'do-blr',
			domain_name VARCHAR(255)
				CONSTRAINT deployment_uq_domain_name UNIQUE
				CONSTRAINT deployment_chk_domain_name_is_lower_case CHECK(
					domain_name = LOWER(domain_name)
				),
			horizontal_scale SMALLINT NOT NULL
				CONSTRAINT deployment_chk_horizontal_scale_u8 CHECK(
					horizontal_scale >= 0 AND horizontal_scale <= 256
				)
				DEFAULT 1,
			machine_type DEPLOYMENT_MACHINE_TYPE NOT NULL DEFAULT 'small',
			organisation_id BYTEA NOT NULL,
			CONSTRAINT deployment_uq_name_organisation_id
				UNIQUE(name, organisation_id),
			CONSTRAINT deployment_chk_repository_id_is_valid CHECK(
				(
					registry = 'registry.patr.cloud' AND
					image_name IS NULL AND
					repository_id IS NOT NULL
				)
				OR
				(
					registry != 'registry.patr.cloud' AND
					image_name IS NOT NULL AND
					repository_id IS NULL
				)
			)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			deployment_idx_name
		ON
			deployment
		(name);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			deployment_idx_image_name_image_tag
		ON
			deployment
		(image_name, image_tag);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			deployment_idx_registry_image_name_image_tag
		ON
			deployment
		(registry, image_name, image_tag);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE deployment_environment_variable(
			deployment_id BYTEA
				CONSTRAINT deploymment_environment_variable_fk_deployment_id
					REFERENCES deployment(id),
			name VARCHAR(256) NOT NULL,
			value TEXT NOT NULL,
			CONSTRAINT deployment_environment_variable_pk
				PRIMARY KEY(deployment_id, name)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE deployment_request_logs(
			id BYTEA CONSTRAINT deployment_request_logs_pk PRIMARY KEY,
			deployment_id BYTEA NOT NULL
				CONSTRAINT deploymment_environment_variable_fk_deployment_id
					REFERENCES deployment(id),
			ip_address VARCHAR(255) NOT NULL,
			ip_address_location POINT NOT NULL,
			method METHOD NOT NULL,
			domain_name VARCHAR(255) NOT NULL
				CONSTRAINT deployment_request_logs_chk_domain_name_is_lower_case CHECK(
					domain_name = LOWER(domain_name)
				),
			protocol PROTOCOL NOT NULL,
			path TEXT NOT NULL,
			response_time REAL NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE data_center_locations(
			region TEXT CONSTRAINT data_center_locations_pk PRIMARY KEY,
			location POINT NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn initialize_deployment_post(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Finishing up deployment tables initialization");
	query!(
		r#"
		ALTER TABLE deployment
		ADD CONSTRAINT deployment_fk_id_organisation_id
		FOREIGN KEY(id, organisation_id) REFERENCES resource(id, owner_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment_request_logs
		ADD CONSTRAINT deployment_request_logs_fk_id
		FOREIGN KEY(id) REFERENCES resource(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		INSERT INTO data_center_locations
		VALUES
		('aws-us-east-1', ST_POINT(-77.4524237, 38.9940541)::point),
		('aws-us-east-2', ST_POINT(-82.7541337, 40.0946354)::point),
		('aws-us-west-2', ST_POINT(-119.2684488, 45.9174667)::point),
		('aws-eu-west-1', ST_POINT(-6.224503, 53.4056545)::point),
		('aws-eu-west-2', ST_POINT(-0.0609266, 51.5085036)::point),
		('aws-eu-west-3', ST_POINT(2.2976644, 48.6009709)::point),
		('aws-eu-central-1', ST_POINT(8.6303932, 50.0992094)::point),
		('aws-ap-southeast-1', ST_POINT(103.6930643, 1.3218269)::point),
		('aws-ap-southeast-2', ST_POINT(151.1907535, -33.9117717)::point),
		('aws-ap-northeast-1', ST_POINT(139.7459176, 35.617436)::point),
		('aws-ap-northeast-2', ST_POINT(126.8736237, 37.5616592)::point),
		('aws-ap-south-1', ST_POINT(72.9667878, 19.2425503)::point),
		('aws-ca-central-1', ST_POINT(-73.6, 45.5)::point),
		('aws-eu-north-1', ST_POINT(17.8419717, 59.326242)::point),
		('do-tor', ST_POINT(-79.3623, 43.6547)::point),
		('do-sfo', ST_POINT(-121.9753, 37.3417)::point),
		('do-nyc', ST_POINT(-73.981, 40.7597)::point),
		('do-lon', ST_POINT(-0.6289, 51.5225)::point),
		('do-ams', ST_POINT(4.9479, 52.3006)::point),
		('do-sgp', ST_POINT(103.695, 1.32123)::point),
		('do-fra', ST_POINT(8.6843, 50.1188)::point),
		('do-blr', ST_POINT(77.5855, 12.9634)::point);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn create_deployment_with_internal_registry(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
	name: &str,
	repository_id: &[u8],
	image_tag: &str,
	region: &str,
	domain_name: Option<&str>,
	horizontal_scale: u64,
	machine_type: &DeploymentMachineType,
	organisation_id: &[u8],
) -> Result<(), sqlx::Error> {
	if let Some(domain) = domain_name {
		query!(
			r#"
			INSERT INTO
				deployment
			VALUES
				(
					$1,
					$2,
					'registry.patr.cloud',
					$3,
					NULL,
					$4,
					'created',
					NULL,
					NULL,
					$5,
					$6,
					$7,
					$8,
					$9
				);
			"#,
			deployment_id,
			name,
			repository_id,
			image_tag,
			region,
			domain,
			horizontal_scale as i16,
			machine_type as _,
			organisation_id,
		)
		.execute(&mut *connection)
		.await
		.map(|_| ())
	} else {
		query!(
			r#"
			INSERT INTO
				deployment
			VALUES
				(
					$1,
					$2,
					'registry.patr.cloud',
					$3,
					NULL,
					$4,
					'created',
					NULL,
					NULL,
					$5,
					NULL,
					$6,
					$7,
					$8
				);
			"#,
			deployment_id,
			name,
			repository_id,
			image_tag,
			region,
			horizontal_scale as i16,
			machine_type as _,
			organisation_id,
		)
		.execute(&mut *connection)
		.await
		.map(|_| ())
	}
}

pub async fn create_deployment_with_external_registry(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
	name: &str,
	registry: &str,
	image_name: &str,
	image_tag: &str,
	region: &str,
	domain_name: Option<&str>,
	horizontal_scale: u64,
	machine_type: &DeploymentMachineType,
	organisation_id: &[u8],
) -> Result<(), sqlx::Error> {
	if let Some(domain) = domain_name {
		query!(
			r#"
			INSERT INTO
				deployment
			VALUES
				(
					$1,
					$2,
					$3,
					NULL,
					$4,
					$5,
					'created',
					NULL,
					NULL,
					$6,
					$7,
					$8,
					$9,
					$10
				);
			"#,
			deployment_id,
			name,
			registry,
			image_name,
			image_tag,
			region,
			domain,
			horizontal_scale as i16,
			machine_type as _,
			organisation_id,
		)
		.execute(&mut *connection)
		.await
		.map(|_| ())
	} else {
		query!(
			r#"
			INSERT INTO
				deployment
			VALUES
				(
					$1,
					$2,
					$3,
					NULL,
					$4,
					$5,
					'created',
					NULL,
					NULL,
					$6,
					NULL,
					$7,
					$8,
					$9
				);
			"#,
			deployment_id,
			name,
			registry,
			image_name,
			image_tag,
			region,
			horizontal_scale as i16,
			machine_type as _,
			organisation_id,
		)
		.execute(&mut *connection)
		.await
		.map(|_| ())
	}
}

pub async fn get_deployments_by_image_name_and_tag_for_organisation(
	connection: &mut <Database as sqlx::Database>::Connection,
	image_name: &str,
	image_tag: &str,
	organisation_id: &[u8],
) -> Result<Vec<Deployment>, sqlx::Error> {
	let rows = query_as!(
		Deployment,
		r#"
		SELECT
			deployment.id,
			deployment.name,
			deployment.registry,
			deployment.repository_id,
			deployment.image_name,
			deployment.image_tag,
			deployment.status as "status: _",
			deployment.deployed_image,
			deployment.digitalocean_app_id,
			deployment.region,
			deployment.domain_name,
			deployment.horizontal_scale,
			deployment.machine_type as "machine_type: _",
			deployment.organisation_id
		FROM
			deployment
		LEFT JOIN
			docker_registry_repository
		ON
			docker_registry_repository.id = deployment.repository_id
		WHERE
			(
				(
					deployment.registry = 'registry.patr.cloud' AND
					docker_registry_repository.name = $1
				) OR
				(
					deployment.registry != 'registry.patr.cloud' AND
					deployment.image_name = $1
				)
			) AND
			deployment.image_tag = $2 AND
			deployment.organisation_id = $3 AND
			deployment.status != 'deleted';
		"#,
		image_name,
		image_tag,
		organisation_id
	)
	.fetch_all(&mut *connection)
	.await?;

	Ok(rows)
}

pub async fn get_deployments_for_organisation(
	connection: &mut <Database as sqlx::Database>::Connection,
	organisation_id: &[u8],
) -> Result<Vec<Deployment>, sqlx::Error> {
	let rows = query_as!(
		Deployment,
		r#"
		SELECT
			id,
			name,
			registry,
			repository_id,
			image_name,
			image_tag,
			status as "status: _",
			deployed_image,
			digitalocean_app_id,
			region,
			domain_name,
			horizontal_scale,
			machine_type as "machine_type: _",
			organisation_id
		FROM
			deployment
		WHERE
			organisation_id = $1 AND
			status != 'deleted';
		"#,
		organisation_id
	)
	.fetch_all(&mut *connection)
	.await?;

	Ok(rows)
}

pub async fn get_deployment_by_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
) -> Result<Option<Deployment>, sqlx::Error> {
	let row = query_as!(
		Deployment,
		r#"
		SELECT
			id,
			name,
			registry,
			repository_id,
			image_name,
			image_tag,
			status as "status: _",
			deployed_image,
			digitalocean_app_id,
			region,
			domain_name,
			horizontal_scale,
			machine_type as "machine_type: _",
			organisation_id
		FROM
			deployment
		WHERE
			id = $1 AND
			status != 'deleted';
		"#,
		deployment_id
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.next();

	Ok(row)
}

pub async fn update_deployment_deployed_image(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
	deployed_image: Option<&str>,
) -> Result<(), sqlx::Error> {
	if let Some(deployed_image) = deployed_image {
		query!(
			r#"
			UPDATE
				deployment
			SET
				deployed_image = $1
			WHERE
				id = $2;
			"#,
			deployed_image,
			deployment_id
		)
		.execute(&mut *connection)
		.await
		.map(|_| ())
	} else {
		query!(
			r#"
			UPDATE
				deployment
			SET
				deployed_image = NULL
			WHERE
				id = $1;
			"#,
			deployment_id
		)
		.execute(&mut *connection)
		.await
		.map(|_| ())
	}
}

pub async fn update_digitalocean_app_id_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	app_deployment_id: &str,
	deployment_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			deployment
		SET
			digitalocean_app_id = $1
		WHERE
			id = $2;
		"#,
		app_deployment_id,
		deployment_id
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_deployment_status(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
	status: &DeploymentStatus,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			deployment
		SET
			status = $1
		WHERE
			id = $2;
		"#,
		status as _,
		deployment_id
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_environment_variables_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
) -> Result<Vec<(String, String)>, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT
			*
		FROM
			deployment_environment_variable
		WHERE
			deployment_id = $1;
		"#,
		deployment_id
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| (row.name, row.value))
	.collect();
	Ok(rows)
}

pub async fn add_environment_variable_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
	key: &str,
	value: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO 
			deployment_environment_variable
		VALUES
			($1, $2, $3);
		"#,
		deployment_id,
		key,
		value
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn remove_all_environment_variables_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			deployment_environment_variable
		WHERE
			deployment_id = $1;
		"#,
		deployment_id,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn set_domain_name_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
	domain_name: Option<&str>,
) -> Result<(), sqlx::Error> {
	if let Some(domain_name) = domain_name {
		query!(
			r#"
			UPDATE
				deployment
			SET
				domain_name = $1
			WHERE
				id = $2;
			"#,
			domain_name,
			deployment_id,
		)
		.execute(&mut *connection)
		.await
		.map(|_| ())
	} else {
		query!(
			r#"
			UPDATE
				deployment
			SET
				domain_name = NULL
			WHERE
				id = $1;
			"#,
			deployment_id,
		)
		.execute(&mut *connection)
		.await
		.map(|_| ())
	}
}

pub async fn set_horizontal_scale_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
	horizontal_scale: u64,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			deployment
		SET
			horizontal_scale = $1
		WHERE
			id = $2;
		"#,
		horizontal_scale as i16,
		deployment_id,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn set_machine_type_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
	machine_type: &DeploymentMachineType,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			deployment
		SET
			machine_type = $1
		WHERE
			id = $2;
		"#,
		machine_type as _,
		deployment_id,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn create_log_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	id: &[u8],
	deployment_id: &[u8],
	ip_address: &str,
	ip_address_latitude: f64,
	ip_address_longitude: f64,
	method: Method,
	domain: &str,
	protocol: Protocol,
	path: &str,
	response_time: f64,
) -> Result<(), sqlx::Error> {
	// let coordinates: geo::Geometry<f64> =
	// 	geo::Point::new(ip_address_latitude, ip_address_longitude).into();
	query!(
		r#"
		INSERT INTO 
			deployment_request_logs
		VALUES
			($1, $2, $3, ST_POINT($4, $5)::point, $6, $7, $8, $9, $10);
		"#,
		id,
		deployment_id,
		ip_address,
		ip_address_latitude,
		ip_address_longitude,
		method as Method,
		domain,
		protocol as Protocol,
		path,
		response_time as f32
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_recommended_data_center(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
) -> Result<Vec<AvgDistance>, sqlx::Error> {
	let rows = query_as!(
		AvgDistance,
		r#"
		SELECT
  			data_center_locations.region,
			AVG(st_distancespheroid(
				ST_GeomFromText('POINT(' || deployment_request_logs.ip_address_location[0]::text || ' ' || deployment_request_logs.ip_address_location[1]::text || ')', 4326),
				ST_GeomFromText('POINT(' || data_center_locations.location[0]::text || ' ' || data_center_locations.location[1] || ')', 4326),
    		'SPHEROID["WGS84",6378137,298.257223563]'
			))::NUMERIC(10,2) as "avg_distance!: f64"
		FROM
  			data_center_locations,
  			deployment_request_logs
		INNER JOIN deployment
		ON
			deployment.id = deployment_request_logs.deployment_id
		WHERE
			deployment_id = $1
		GROUP BY
			data_center_locations.region
		ORDER BY
			"avg_distance!: f64";
		"#,
		deployment_id
	)
	.fetch_all(&mut *connection)
	.await?;

	Ok(rows)
}