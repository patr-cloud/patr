use axum::http::StatusCode;
use models::{
	api::{workspace::deployment::*, WithId},
	ErrorType,
};
use regex::Regex;
use time::OffsetDateTime;

use crate::prelude::*;

/// Create a deployment
///
/// # Parameters
/// - `name` - The name of the deployment
/// - `registry` - The registry to use for the deployment
/// - `image_tag` - The tag of the image to deploy
/// - `region` - The region to deploy the deployment in
/// - `machine_type` - The machine type to use for the deployment
/// - `running_details` - The running details of the deployment
/// - `deploy_on_push` - Whether to deploy on push
///
/// # Returns
/// - `id` - The ID of the created deployment
pub async fn create_deployment(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: CreateDeploymentPath { workspace_id },
				query: _,
				headers,
				body:
					CreateDeploymentRequestProcessed {
						name,
						registry,
						image_tag,
						region,
						machine_type,
						running_details:
							DeploymentRunningDetails {
								deploy_on_push,
								min_horizontal_scale,
								max_horizontal_scale,
								ports,
								environment_variables,
								startup_probe,
								liveness_probe,
								config_mounts,
								volumes,
							},
						deploy_on_create,
					},
			},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, CreateDeploymentRequest>,
) -> Result<AppResponse<CreateDeploymentRequest>, ErrorType> {
	info!("Starting: Create deployment");

	// Check if a deployment with same name exists
	let deployment_exist = query!(
		r#"
		SELECT
			name
		FROM
			deployment
		WHERE
			workspace_id = $1 AND
			name = $2
		"#,
		workspace_id as _,
		&name
	)
	.fetch_optional(&mut **database)
	.await?
	.is_some();

	if deployment_exist {
		return Err(ErrorType::ResourceAlreadyExists);
	}

	todo!("Get limit on resource creation, max deployment and max volume depending on the users patr plan if not a byoc user");

	let created_time = OffsetDateTime::now_utc();
	let deployment_id = loop {
		let uuid = Uuid::new_v4();

		let exists = query!(
			r#"
			SELECT
				*
			FROM
				resource
			WHERE
				id = $1;
			"#,
			uuid as _
		)
		.fetch_optional(&mut **database)
		.await?
		.is_some();

		if !exists {
			break uuid;
		}
	};

	// BEGIN DEFERRED CONSTRAINT
	query!(
		r#"
		SET CONSTRAINTS ALL DEFERRED;
		"#,
	)
	.execute(&mut **database)
	.await?;

	match registry {
		DeploymentRegistry::PatrRegistry {
			registry: _,
			repository_id,
		} => {
			// Creating database record with internal registry
			query!(
				r#"
				INSERT INTO
					deployment(
						id,
						name,
						registry,
						repository_id,
						image_name,
						image_tag,
						status,
						workspace_id,
						runner,
						min_horizontal_scale,
						max_horizontal_scale,
						machine_type,
						deploy_on_push,
						startup_probe_port,
						startup_probe_path,
						startup_probe_port_type,
						liveness_probe_port,
						liveness_probe_path,
						liveness_probe_port_type
					)
				VALUES
					(
						$1,
						$2,
						'registry.patr.cloud',
						$3,
						NULL,
						$4,
						'created',
						$5,
						$6,
						$7,
						$8,
						$9,
						$10,
						$11,
						$12,
						$13,
						$14,
						$15,
						$16
					);
				"#,
				deployment_id as _,
				name as _,
				repository_id as _,
				image_tag.as_ref(),
				workspace_id as _,
				region as _,
				min_horizontal_scale as i32,
				max_horizontal_scale as i32,
				machine_type as _,
				deploy_on_push,
				startup_probe.map(|probe| probe.port as i32),
				startup_probe.map(|probe| probe.path),
				startup_probe.map(|_| ExposedPortType::Http) as _,
				liveness_probe.map(|probe| probe.port as i32),
				liveness_probe.map(|probe| probe.path),
				liveness_probe.map(|_| ExposedPortType::Http) as _,
			)
			.execute(&mut **database)
			.await?;
		}
		DeploymentRegistry::ExternalRegistry {
			registry,
			image_name,
		} => {
			// Creating database record with external registry
			query!(
				r#"
				INSERT INTO
					deployment(
						id,
						name,
						registry,
						repository_id,
						image_name,
						image_tag,
						status,
						workspace_id,
						runner,
						min_horizontal_scale,
						max_horizontal_scale,
						machine_type,
						deploy_on_push,
						startup_probe_port,
						startup_probe_path,
						startup_probe_port_type,
						liveness_probe_port,
						liveness_probe_path,
						liveness_probe_port_type
					)
				VALUES
					(
						$1,
						$2,
						$3,
						NULL,
						$4,
						$5,
						'created',
						$6,
						$7,
						$8,
						$9,
						$10,
						$11,
						$12,
						$13,
						$14,
						$15,
						$16,
						$17
					);
				"#,
				deployment_id as _,
				name as _,
				registry,
				image_name,
				image_tag.as_ref(),
				workspace_id as _,
				region as _,
				min_horizontal_scale as i32,
				max_horizontal_scale as i32,
				machine_type as _,
				deploy_on_push,
				startup_probe.map(|probe| probe.port as i32),
				startup_probe.map(|probe| probe.path),
				startup_probe.map(|_| ExposedPortType::Http) as _,
				liveness_probe.map(|probe| probe.port as i32),
				liveness_probe.map(|probe| probe.path),
				liveness_probe.map(|_| ExposedPortType::Http) as _,
			)
			.execute(&mut **database)
			.await?;
		}
	}

	for (port, port_type) in &ports {
		// Adding exposed port entry to database
		query!(
			r#"
			INSERT INTO 
				deployment_exposed_port(
					deployment_id,
					port,
					port_type
				)
			VALUES
				($1, $2, $3);
			"#,
			deployment_id as _,
			port.value() as i32,
			port_type as _
		)
		.execute(&mut **database)
		.await?;
	}

	// END DEFERRED CONSTRAINT
	query!(
		r#"
		SET CONSTRAINTS ALL IMMEDIATE;
		"#,
	)
	.execute(&mut **database)
	.await?;

	for (key, value) in &environment_variables {
		// Adding environment variable entry to database
		match value {
			EnvironmentVariableValue::String(value) => {
				query!(
					r#"
					INSERT INTO 
						deployment_environment_variable(
							deployment_id,
							name,
							value,
							secret_id
						)
					VALUES
						($1, $2, $3, $4);
					"#,
					deployment_id as _,
					key,
					Some(value),
					None::<Uuid> as _
				)
				.execute(&mut **database)
				.await?;
			}
			EnvironmentVariableValue::Secret {
				from_secret: secret_id,
			} => {
				query!(
					r#"
					INSERT INTO 
						deployment_environment_variable(
							deployment_id,
							name,
							value,
							secret_id
						)
					VALUES
						($1, $2, $3, $4);
					"#,
					deployment_id as _,
					key,
					None::<String>,
					Some(secret_id) as _,
				)
				.execute(&mut **database)
				.await?;
			}
		}
	}

	for (path, file) in &config_mounts {
		// Decoding config file from base64 to byte array
		query!(
			r#"
			INSERT INTO 
				deployment_config_mounts(
					path,
					file,
					deployment_id
				)
			VALUES
				($1, $2, $3);
			"#,
			path,
			file as &[u8],
			deployment_id as _,
		)
		.execute(&mut **database)
		.await?;
	}

	for (name, volume) in &volumes {
		let volume_id = loop {
			let uuid = Uuid::new_v4();

			let exists = query!(
				r#"
				SELECT
					*
				FROM
					resource
				WHERE
					id = $1;
				"#,
				uuid as _
			)
			.fetch_optional(&mut **database)
			.await?
			.is_some();

			if !exists {
				break uuid;
			}
		};

		query!(
			r#"
			INSERT INTO
				resource(
					id,
					resource_type_id,
					owner_id,
					created
				)
			VALUES
				($1, (SELECT id FROM resource_type WHERE name = 'deployment_volume'), $2, $3);
			"#,
			volume_id as _,
			workspace_id as _,
			created_time
		)
		.execute(&mut **database)
		.await?;

		query!(
			r#"
			INSERT INTO 
				deployment_volume(
					id,
					name,
					deployment_id,
					volume_size,
					volume_mount_path
				)
			VALUES
				($1, $2, $3, $4, $5);
			"#,
			volume_id as _,
			name,
			deployment_id as _,
			volume.size as i32,
			volume.path,
		)
		.execute(&mut **database)
		.await?;
	}

	todo!("Generate audit log");
	todo!("update_cloudflare_kv_for_deployment");

	if let DeploymentRegistry::PatrRegistry { repository_id, .. } = &registry {
		let digest = query!(
			r#"
			SELECT
				manifest_digest
			FROM
				container_registry_repository_manifest
			WHERE
				repository_id = $1
			ORDER BY
				created DESC
			LIMIT 1;
			"#,
			repository_id as _
		)
		.fetch_optional(&mut **database)
		.await?
		.map(|row| row.manifest_digest);

		if let Some(digest) = digest {
			query!(
				r#"
				INSERT INTO
					deployment_deploy_history(
						deployment_id,
						image_digest,
						repository_id,
						created
					)
				VALUES
					($1, $2, $3, $4)
				ON CONFLICT
					(deployment_id, image_digest)
				DO NOTHING;
				"#,
				deployment_id as _,
				digest as _,
				repository_id as _,
				created_time as _,
			)
			.execute(&mut **database)
			.await?;

			query!(
				r#"
				INSERT INTO
					deployment_deploy_history(
						deployment_id,
						image_digest,
						repository_id,
						created
					)
				VALUES
					($1, $2, $3, $4)
				ON CONFLICT
					(deployment_id, image_digest)
				DO NOTHING;
				"#,
				deployment_id as _,
				digest as _,
				repository_id as _,
				created_time as _,
			)
			.execute(&mut **database)
			.await?;
		}
	}

	todo!("Deployment metric");

	AppResponse::builder()
		.body(CreateDeploymentResponse {
			id: WithId::new(deployment_id, ()),
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
