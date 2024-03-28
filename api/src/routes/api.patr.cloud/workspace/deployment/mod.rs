use std::{cmp::Ordering, collections::BTreeMap};

use axum::{http::StatusCode, Router};
use futures::sink::With;
use models::{
	api::{
		workspace::{
			container_registry::*,
			deployment::*,
			infrastructure::{deployment::*, managed_url::*},
			region::*,
		},
		WithId,
	},
	utils::StringifiedU16,
	ApiRequest,
	ErrorType,
};
use time::OffsetDateTime;

use crate::{models::deployment::MACHINE_TYPES, prelude::*, utils::validator};

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.mount_endpoint(machine_type, state)
		.mount_auth_endpoint(list_deployment, state)
		.mount_auth_endpoint(list_deployment_history, state)
		.mount_auth_endpoint(create_deployment, state)
		.mount_auth_endpoint(get_deployment_info, state)
		.mount_auth_endpoint(start_deployment, state)
		.mount_auth_endpoint(stop_deployment, state)
		.mount_auth_endpoint(revert_deployment, state)
		.mount_auth_endpoint(get_deployment_log, state)
		.mount_auth_endpoint(delete_deployment, state)
		.mount_auth_endpoint(update_deployment, state)
		.mount_auth_endpoint(list_linked_url, state)
		.mount_auth_endpoint(get_deployment_metric, state)
}

async fn machine_type(
	AppRequest {
		request: ProcessedApiRequest {
			path,
			query: _,
			headers,
			body,
		},
		database,
		redis: _,
		client_ip: _,
		config,
	}: AppRequest<'_, ListAllDeploymentMachineTypeRequest>,
) -> Result<AppResponse<ListAllDeploymentMachineTypeRequest>, ErrorType> {
	info!("Starting: List deployments");

	// LOGIC

	AppResponse::builder()
		.body(ListAllDeploymentMachineTypeResponse {
			machine_types: todo!(),
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn list_deployment(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ListDeploymentPath { workspace_id },
				query: _,
				headers,
				body,
			},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, ListDeploymentRequest>,
) -> Result<AppResponse<ListDeploymentRequest>, ErrorType> {
	info!("Starting: List deployments");

	let deployments = query!(
		r#"
		SELECT
			id,
			name,
			registry,
			repository_id,
			image_name,
			image_tag,
			status,
			region,
			machine_type,
			current_live_digest
		FROM
			deployment
		WHERE
			workspace_id = $1 AND
			status != 'deleted';
		"#,
		workspace_id as _
	)
	.fetch_optional(&mut **database)
	.await?
	.into_iter()
	.map(|deployment| {
		WithId::new(
			deployment.id.into(),
			Deployment {
				name: deployment.name,
				registry: if deployment.registry == PatrRegistry.to_string() {
					DeploymentRegistry::PatrRegistry {
						registry: PatrRegistry,
						repository_id: deployment.repository_id.unwrap().into(),
					}
				} else {
					DeploymentRegistry::ExternalRegistry {
						registry: deployment.registry,
						image_name: deployment.image_name.unwrap().into(),
					}
				},
				image_tag: deployment.image_tag,
				status: deployment.status,
				region: deployment.region.into(),
				machine_type: deployment.machine_type.into(),
				current_live_digest: deployment.current_live_digest,
			},
		)
	})
	.collect();

	todo!("Filter out deployments that are not supposed to be viewed");

	AppResponse::builder()
		.body(ListDeploymentResponse { deployments })
		.headers(ListDeploymentResponseHeaders {
			total_count: todo!(),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn list_deployment_history(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ListDeploymentHistoryPath {
					workspace_id,
					deployment_id,
				},
				query: _,
				headers,
				body,
			},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, ListDeploymentHistoryRequest>,
) -> Result<AppResponse<ListDeploymentHistoryRequest>, ErrorType> {
	info!("Starting: List deployment history");

	let deploys = query!(
		r#"
		SELECT 
			image_digest,
			created
		FROM
			deployment_deploy_history
		WHERE
			deployment_id = $1;
		"#,
		deployment_id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::ResourceDoesNotExist)
	.into_iter()
	.map(|deploy| DeploymentDeployHistory {
		image_digest: deploy.image_digest,
		created: deploy.created,
	})
	.collect();

	AppResponse::builder()
		.body(ListDeploymentHistoryResponse { deploys })
		.headers(ListDeploymentHistoryResponseHeaders {
			total_count: todo!(),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn create_deployment(
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
						running_details,
						deploy_on_push,
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
			workspace_id = $1;
		"#,
		workspace_id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	.is_some();

	if deployment_exist {
		return Err(ErrorType::ResourceAlreadyExists);
	}

	// Validations
	if image_tag.is_empty() {
		return Err(ErrorType::WrongParameters);
	}

	if running_details.ports.is_empty() {
		return Err(ErrorType::WrongParameters);
	}

	if let DeploymentRegistry::ExternalRegistry { image_name, .. } = registry {
		if !validator::is_docker_image_name_valid(&image_name.trim()) {
			return Err(ErrorType::InvalidImageName);
		}
	}

	if !validator::is_deployment_name_valid(&name.trim()) {
		return Err(ErrorType::InvalidDeploymentName);
	}

	// Check region if active
	let region_details = query!(
		r#"
		SELECT
			status
		FROM
			region
		WHERE
			id = $1;
		"#,
		region as _,
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::server_error("Could not get region details"))?;

	todo!("Filter all the patr regions");

	if !(region_details.status == RegionStatus::Active || todo!("Check if byoc region")) {
		return Err(ErrorType::RegionNotActive);
	}

	// Check creation limits
	// If deploy on Patr then the user is only allowed to create resources depending
	// on their current active plan quota
	if todo!("check if free user") {
		let current_deployment_count = query!(
			r#"
			SELECT
				COUNT(id)
			FROM
				deployment
			WHERE
				workspace_id = $1;
			"#,
			workspace_id as _,
		)
		.fetch_optional(&mut **database)
		.await?
		.map(|row| row.count.unwrap_or(0))
		.ok_or(ErrorType::server_error(
			"Could not get total deployment count",
		))?;

		todo!("Check card details");

		if current_deployment_count.into() >= constants::DEFAULT_DEPLOYMENT_LIMIT {
			return Err(ErrorType::FreeLimitExceeded);
		}

		// only basic machine type is allowed under free plan
		todo!("Check");
		let machine_type_to_be_deployed = MACHINE_TYPES
			.get()
			.and_then(|machines| machines.get(machine_type))
			.status(500)?;

		if machine_type_to_be_deployed != &(1, 2) {
			return Err(ErrorType::FreeLimitExceeded);
		}

		if running_details.max_horizontal_scale > 1 || running_details.min_horizontal_scale > 1 {
			return Err(ErrorType::FreeLimitExceeded);
		}

		let volume_size = running_details
			.volumes
			.iter()
			.map(|(_, volume)| volume.size as u32)
			.sum::<u32>();

		let volume_size_in_byte = volume_size as usize * 1024 * 1024 * 1024;
		if volume_size_in_byte > constants::VOLUME_STORAGE_IN_BYTE {
			return Err(ErrorType::FreeLimitExceeded);
		}
	}

	todo!("Get limit on resource creation, max deployment and max volume depending on the users patr plan if not a byoc user");

	let created_time = OffsetDateTime::now_utc();

	todo!("Have a funcion to generate new distinct ID");
	let deployment_id = Uuid::new_v4();

	// Create resource
	let resource_type_id: Uuid = todo!("Get resource ID for a deployment");
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
			($1, $2, $3, $4);
		"#,
		deployment_id as _,
		resource_type_id as _,
		workspace_id as _,
		created_time
	)
	.execute(&mut **database)
	.await?;

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
						region,
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
				image_tag,
				workspace_id as _,
				region as _,
				running_details.min_horizontal_scale as i32,
				running_details.max_horizontal_scale as i32,
				machine_type as _,
				deploy_on_push,
				running_details.startup_probe.map(|probe| probe.port as i32),
				running_details
					.startup_probe
					.map(|probe| probe.path.as_str()),
				running_details.startup_probe.map(|_| ExposedPortType::Http) as _,
				running_details
					.liveness_probe
					.map(|probe| probe.port as i32),
				running_details
					.liveness_probe
					.map(|probe| probe.path.as_str()),
				running_details
					.liveness_probe
					.map(|_| ExposedPortType::Http) as _,
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
						region,
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
				image_tag,
				workspace_id as _,
				region as _,
				running_details.min_horizontal_scale as i32,
				running_details.max_horizontal_scale as i32,
				machine_type as _,
				deploy_on_push,
				running_details.startup_probe.map(|probe| probe.port as i32),
				running_details
					.startup_probe
					.map(|probe| probe.path.as_str()),
				running_details.startup_probe.map(|_| ExposedPortType::Http) as _,
				running_details
					.liveness_probe
					.map(|probe| probe.port as i32),
				running_details
					.liveness_probe
					.map(|probe| probe.path.as_str()),
				running_details
					.liveness_probe
					.map(|_| ExposedPortType::Http) as _,
			)
			.execute(&mut **database)
			.await?;
		}
	}

	for (port, port_type) in &running_details.ports {
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

	for (key, value) in &running_details.environment_variables {
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
					None as _
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
					None,
					Some(secret_id) as _,
				)
				.execute(&mut **database)
				.await?;
			}
		}
	}

	for (path, file) in &running_details.config_mounts {
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

	for (name, volume) in &running_details.volumes {
		todo!("Have a funcion to generate new distinct ID");
		let volume_id = Uuid::new_v4();

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
				($1, $2, $3, $4);
			"#,
			volume_id as _,
			resource_type_id as _,
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

	todo!("Deployment metric");

	AppResponse::builder()
		.body(CreateDeploymentResponse {
			id: WithId::new(deployment_id, None),
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn get_deployment_info(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: GetDeploymentInfoPath {
					workspace_id,
					deployment_id,
				},
				query: _,
				headers,
				body,
			},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, GetDeploymentInfoRequest>,
) -> Result<AppResponse<GetDeploymentInfoRequest>, ErrorType> {
	info!("Starting: Get deployment info");

	let deployment_ports = query!(
		r#"
		SELECT
			port,
			port_type as "port_type: ExposedPortType"
		FROM
			deployment_exposed_port
		WHERE
			deployment_id = $1;
		"#,
		deployment_id as _
	)
	.fetch_optional(&mut **database)
	.await?
	.into_iter()
	.map(|row| (StringifiedU16::new(row.port as u16), row.port_type))
	.collect();

	let deployment_env_variables = query!(
		r#"
		SELECT
			name,
			value,
			secret_id
		FROM
			deployment_environment_variable
		WHERE
			deployment_id = $1;
		"#,
		deployment_id as _
	)
	.fetch_optional(&mut **database)
	.await?
	.into_iter()
	.filter_map(|env| match (env.value, env.secret_id) {
		(Some(value), None) => Some((env.name, EnvironmentVariableValue::String(value))),
		(None, Some(secret_id)) => Some((
			env.name,
			EnvironmentVariableValue::Secret {
				from_secret: secret_id.into(),
			},
		)),
		_ => None,
	})
	.collect();

	let deployment_config_mounts = query!(
		r#"
		SELECT
			path,
			file
		FROM
			deployment_config_mounts
		WHERE
			deployment_id = $1;
		"#,
		deployment_id as _
	)
	.fetch_optional(&mut **database)
	.await?
	.into_iter()
	.map(|mount| (mount.path, mount.file.into()))
	.collect();

	let deployment_volumes = query!(
		r#"
		SELECT
			id,
			name,
			volume_size,
			volume_mount_path
		FROM
			deployment_volume
		WHERE
			deployment_id = $1 AND
			deleted IS NULL;
		"#,
		deployment_id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	.into_iter()
	.map(|volume| {
		(
			volume.name,
			DeploymentVolume {
				path: volume.volume_mount_path,
				size: volume.volume_size as u16,
			},
		)
	})
	.collect();

	let deployment = query!(
		r#"
		SELECT
			id,
			name,
			registry,
			repository_id,
			image_name,
			image_tag,
			status,
			workspace_id,
			region,
			min_horizontal_scale,
			max_horizontal_scale,
			machine_type,
			deploy_on_push,
			startup_probe_port,
			startup_probe_path,
			liveness_probe_port,
			liveness_probe_path,
			current_live_digest
		FROM
			deployment
		WHERE
			id = $1 AND
			status != 'deleted';
		"#,
		deployment_id as _
	)
	.fetch_optional(&mut **database)
	.await?
	.map(|deployment| GetDeploymentInfoResponse {
		deployment: WithId::new(
			deployment.id.into(),
			Deployment {
				name: deployment.name,
				registry: if deployment.registry == PatrRegistry.to_string() {
					DeploymentRegistry::PatrRegistry {
						registry: PatrRegistry,
						repository_id: deployment.repository_id.unwrap().into(),
					}
				} else {
					DeploymentRegistry::ExternalRegistry {
						registry: deployment.registry,
						image_name: deployment.image_name.unwrap().into(),
					}
				},
				image_tag: deployment.image_tag.into(),
				status: deployment.status,
				region: deployment.region.into(),
				machine_type: deployment.machine_type.into(),
				current_live_digest: deployment.current_live_digest,
			},
		),
		running_details: DeploymentRunningDetails {
			deploy_on_push: deployment.deploy_on_push,
			min_horizontal_scale: deployment.min_horizontal_scale as u16,
			max_horizontal_scale: deployment.max_horizontal_scale as u16,
			ports: deployment_ports,
			environment_variables: deployment_env_variables,
			startup_probe: Some(DeploymentProbe {
				port: deployment.startup_probe_port.unwrap() as u16,
				path: deployment.startup_probe_path.unwrap(),
			}),
			liveness_probe: Some(DeploymentProbe {
				port: deployment.liveness_probe_port.unwrap() as u16,
				path: deployment.liveness_probe_path.unwrap(),
			}),
			config_mounts: deployment_config_mounts,
			volumes: deployment_volumes,
		},
	})
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	AppResponse::builder()
		.body(deployment)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn start_deployment(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: StartDeploymentPath {
					workspace_id,
					deployment_id,
				},
				query: _,
				headers,
				body,
			},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, StartDeploymentRequest>,
) -> Result<AppResponse<StartDeploymentRequest>, ErrorType> {
	info!("Starting: Start deployment");

	let now = OffsetDateTime::now_utc();

	let (registry, image_tag, region) = query!(
		r#"
		SELECT
			registry,
			repository_id,
			image_name,
			image_tag,
			region
		FROM
			deployment
		WHERE
			id = $1 AND
			status != 'deleted';
		"#,
		deployment_id as _
	)
	.fetch_optional(&mut **database)
	.await?
	.map(|deployment| {
		let registry = if deployment.registry == PatrRegistry.to_string() {
			DeploymentRegistry::PatrRegistry {
				registry: PatrRegistry,
				repository_id: deployment.repository_id.unwrap().into(),
			}
		} else {
			DeploymentRegistry::ExternalRegistry {
				registry: deployment.registry,
				image_name: deployment.image_name.unwrap().into(),
			}
		};
		(registry, deployment.image_tag, deployment.region)
	})
	.ok_or(ErrorType::ResourceDoesNotExist)?;

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
			// Check if digest is already in deployment_deploy_history table
			let deployment_deploy_history = query_as!(
				DeploymentDeployHistory,
				r#"
				SELECT 
					image_digest,
					created as "created: _"
				FROM
					deployment_deploy_history
				WHERE
					image_digest = $1;
				"#,
				digest as _,
			)
			.fetch_optional(&mut **database)
			.await?;

			// If not, add it to the table
			if deployment_deploy_history.is_none() {
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
					now as _,
				)
				.execute(&mut **database)
				.await
				.map(|_| ());
			}
		}
	}

	let (image_name, digest) = match registry {
		DeploymentRegistry::PatrRegistry {
			registry: _,
			repository_id,
		} => {
			let digest = query!(
				r#"
				SELECT
					manifest_digest
				FROM
					container_registry_repository_tag
				WHERE
					repository_id = $1 AND
					tag = $2;
				"#,
				repository_id as _,
				image_tag
			)
			.fetch_optional(&mut **database)
			.await?
			.map(|row| row.manifest_digest);

			let repository_name = query!(
				r#"
				SELECT
					name
				FROM
					container_registry_repository
				WHERE
					id = $1 AND
					deleted IS NULL;
				"#,
				repository_id as _
			)
			.fetch_optional(&mut **database)
			.await?
			.map(|repo| repo.name)
			.ok_or(ErrorType::ResourceDoesNotExist)?;

			if let Some(digest) = digest {
				Ok((
					format!(
						"{}/{}/{}",
						todo!("config.docker_registry.registry_url"),
						workspace_id,
						repository_name
					),
					Some(digest),
				))
			} else {
				Ok((
					format!(
						"{}/{}/{}:{}",
						todo!("config.docker_registry.registry_url"),
						workspace_id,
						repository_name,
						image_tag
					),
					None,
				))
			}
		}
		DeploymentRegistry::ExternalRegistry {
			registry,
			image_name,
		} => match registry.as_str() {
			"registry.hub.docker.com" | "hub.docker.com" | "index.docker.io" | "docker.io" | "" => {
				Ok((format!("docker.io/{}:{}", image_name, image_tag), None))
			}
			_ => Ok((format!("{}/{}:{}", registry, image_name, image_tag), None)),
		},
	}?;

	// Update status to deploying
	query!(
		r#"
		UPDATE
			deployment
		SET
			status = $1
		WHERE
			id = $2;
		"#,
		DeploymentStatus::Deploying as _,
		deployment_id as _
	)
	.execute(&mut **database)
	.await?;

	let volumes = query!(
		r#"
		SELECT
			id,
			name,
			volume_size,
			volume_mount_path
		FROM
			deployment_volume
		WHERE
			deployment_id = $1 AND
			deleted IS NULL;
		"#,
		deployment_id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	.into_iter()
	.map(|volume| {
		(
			volume.name,
			DeploymentVolume {
				path: volume.volume_mount_path,
				size: volume.volume_size as u16,
			},
		)
	})
	.collect();

	if todo!("If deployment on patr region") {
		for volume in &volumes {
			todo!("Start usage history for volume")
		}

		todo!("Start usage history for deployment");
	}

	todo!("Audit log");

	AppResponse::builder()
		.body(StartDeploymentResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn stop_deployment(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: StopDeploymentPath {
					workspace_id,
					deployment_id,
				},
				query: _,
				headers,
				body,
			},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, StopDeploymentRequest>,
) -> Result<AppResponse<StopDeploymentRequest>, ErrorType> {
	info!("Starting: Stop deployment");

	// Updating deployment status
	query!(
		r#"
		UPDATE
			deployment
		SET
			status = $1
		WHERE
			id = $2;
		"#,
		DeploymentStatus::Stopped as _,
		deployment_id as _
	)
	.execute(&mut **database)
	.await?;

	todo!("Stop deployment usage history");
	todo!("Audit log");

	AppResponse::builder()
		.body(StopDeploymentResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn revert_deployment(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path:
					RevertDeploymentPath {
						workspace_id,
						deployment_id,
						digest,
					},
				query: _,
				headers,
				body,
			},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, RevertDeploymentRequest>,
) -> Result<AppResponse<RevertDeploymentRequest>, ErrorType> {
	info!("Starting: Revert deployment");

	// Check if deployment exists
	query!(
		r#"
		SELECT
			id
		FROM
			deployment
		WHERE
			id = $1 AND
			status != 'deleted';
		"#,
		deployment_id as _
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::ResourceDoesNotExist);

	// Check if the digest is present or not in the deployment_deploy_history
	// table
	query!(
		r#"
		SELECT 
			image_digest,
			created
		FROM
			deployment_deploy_history
		WHERE
			image_digest = $1;
		"#,
		digest as _,
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::ResourceDoesNotExist);

	// Revert the digest
	query!(
		r#"
		UPDATE
			deployment
		SET
			current_live_digest = $1
		WHERE
			id = $2;
		"#,
		digest as _,
		deployment_id as _
	)
	.execute(&mut **database)
	.await
	.map(|_| (ErrorType::server_error("Failed to update deployment")));

	// Set deployment status to deploying
	query!(
		r#"
		UPDATE
			deployment
		SET
			status = $1
		WHERE
			id = $2;
		"#,
		DeploymentStatus::Deploying as _,
		deployment_id as _
	)
	.execute(&mut **database)
	.await?;

	todo!("Audit log");

	AppResponse::builder()
		.body(RevertDeploymentResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn get_deployment_log(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: GetDeploymentLogPath {
					workspace_id,
					deployment_id,
				},
				query: GetDeploymentLogQuery { end_time, limit },
				headers,
				body,
			},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, GetDeploymentLogRequest>,
) -> Result<AppResponse<GetDeploymentLogRequest>, ErrorType> {
	info!("Starting: Get deployment logs");

	// LOGIC

	AppResponse::builder()
		.body(GetDeploymentLogResponse { logs: todo!() })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn delete_deployment(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: DeleteDeploymentPath {
					workspace_id,
					deployment_id,
				},
				query: _,
				headers,
				body,
			},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, DeleteDeploymentRequest>,
) -> Result<AppResponse<DeleteDeploymentRequest>, ErrorType> {
	info!("Starting: Delete deployment");

	// Check if deployment exists
	let deployment = query!(
		r#"
		SELECT
			region
		FROM
			deployment
		WHERE
			id = $1 AND
			status != 'deleted';
		"#,
		deployment_id as _
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	// Check if deployment is using managed URLs
	let managed_urls_exists = query!(
		r#"
		SELECT
			id
		FROM
			managed_url
		WHERE
			managed_url.deployment_id = $1 AND
			managed_url.workspace_id = $2 AND
			managed_url.url_type = 'proxy_to_deployment' AND
			managed_url.deleted IS NULL;
		"#,
		deployment_id as _,
		workspace_id as _
	)
	.fetch_optional(&mut **database)
	.await?
	.is_some();

	if managed_urls_exists {
		return Err(ErrorType::ResourceInUse);
	}

	// Check if deployment is on patr region to stop usage tracking
	if todo!("Check if deployment on patr region") {
		todo!("Stop deployment usage history");

		let volumes: BTreeMap<String, DeploymentVolume> = query!(
			r#"
			SELECT
				id,
				name,
				volume_size,
				volume_mount_path
			FROM
				deployment_volume
			WHERE
				deployment_id = $1 AND
				deleted IS NULL;
			"#,
			deployment_id as _,
		)
		.fetch_optional(&mut **database)
		.await?
		.into_iter()
		.map(|volume| {
			(
				volume.name,
				DeploymentVolume {
					path: volume.volume_mount_path,
					size: volume.volume_size as u16,
				},
			)
		})
		.collect();

		for volume in volumes {
			todo!("Stop volume usage history");
		}
	}

	// Mark deployment deleted in database
	query!(
		r#"
		UPDATE
			deployment
		SET
			deleted = $2,
			status = 'deleted'
		WHERE
			id = $1;
		"#,
		deployment_id as _,
		OffsetDateTime::now_utc()
	)
	.execute(&mut **database)
	.await?;

	todo!("Audit log");

	AppResponse::builder()
		.body(DeleteDeploymentResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn update_deployment(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: UpdateDeploymentPath {
					workspace_id,
					deployment_id,
				},
				query: _,
				headers,
				body:
					UpdateDeploymentRequestProcessed {
						name,
						machine_type,
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
			},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, UpdateDeploymentRequest>,
) -> Result<AppResponse<UpdateDeploymentRequest>, ErrorType> {
	info!("Starting: List linked URLs");

	// Validate if at least value is to be updated
	if name.is_none() &&
		machine_type.is_none() &&
		deploy_on_push.is_none() &&
		min_horizontal_scale.is_none() &&
		max_horizontal_scale.is_none() &&
		ports.is_none() &&
		environment_variables.is_none() &&
		startup_probe.is_none() &&
		liveness_probe.is_none() &&
		config_mounts.is_none() &&
		volumes.is_none()
	{
		return Err(ErrorType::WrongParameters);
	}

	let deployment = query!(
		r#"
		SELECT
			region,
			min_horizontal_scale
		FROM
			deployment
		WHERE
			id = $1 AND
			deleted IS NULL;
		"#,
		deployment_id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	// Get volume size to check limit
	let volume_size = if let Some(volume) = volumes {
		volume
			.iter()
			.map(|(_, volume)| volume.size as u32)
			.sum::<u32>()
	} else {
		0
	};

	if todo!("Check if deployment in patr region and card not added") {
		if let Some(machine_type) = machine_type {
			// only basic machine type is allowed under free plan
			let machine_type_to_be_deployed = MACHINE_TYPES
				.get()
				.and_then(|machines| machines.get(machine_type))
				.status(500)?;

			if machine_type_to_be_deployed != &(1, 2) {
				return Err(ErrorType::FreeLimitExceeded);
			}
		}
		if let Some(max_horizontal_scale) = max_horizontal_scale {
			if max_horizontal_scale > 1 {
				return Err(ErrorType::FreeLimitExceeded);
			}
		}

		if let Some(min_horizontal_scale) = min_horizontal_scale {
			if min_horizontal_scale > 1 {
				return Err(ErrorType::FreeLimitExceeded);
			}
		}

		let volume_size_in_bytes = volume_size as usize * 1024 * 1024 * 1024;
		if volume_size_in_bytes > constants::VOLUME_STORAGE_IN_BYTE {
			return Err(ErrorType::FreeLimitExceeded);
		}
	}

	todo!("Check if any workspace limit for volume is there for user");

	// BEGIN DEFERRED CONSTRAINT
	query!(
		r#"
		SET CONSTRAINTS ALL DEFERRED;
		"#,
	)
	.execute(&mut **database)
	.await?;

	if let Some(ports) = ports {
		if ports.is_empty() {
			return Err(ErrorType::WrongParameters);
		}

		// Updating deployment port in database
		query!(
			r#"
			DELETE FROM
				deployment_exposed_port
			WHERE
				deployment_id = $1;
			"#,
			deployment_id as _,
		)
		.execute(&mut **database)
		.await?;

		for (port, exposed_port_type) in ports {
			// Adding new exposed port entry to database
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
				exposed_port_type as _
			)
			.execute(&mut **database)
			.await?;
		}
	}

	// Updating deployment details
	query!(
		r#"
		UPDATE
			deployment
		SET
			name = COALESCE($1, name),
			machine_type = COALESCE($2, machine_type),
			deploy_on_push = COALESCE($3, deploy_on_push),
			min_horizontal_scale = COALESCE($4, min_horizontal_scale),
			max_horizontal_scale = COALESCE($5, max_horizontal_scale),
			startup_probe_port = (
				CASE
					WHEN $6 = 0 THEN
						NULL
					ELSE
						$6
				END
			),
			startup_probe_path = (
				CASE
					WHEN $6 = 0 THEN
						NULL
					ELSE
						$7
				END
			),
			startup_probe_port_type = (
				CASE
					WHEN $6 = 0 THEN
						NULL
					WHEN $6 IS NULL THEN
						startup_probe_port_type
					ELSE
						'http'::EXPOSED_PORT_TYPE
				END
			),
			liveness_probe_port = (
				CASE
					WHEN $8 = 0 THEN
						NULL
					ELSE
						$8
				END
			),
			liveness_probe_path = (
				CASE
					WHEN $8 = 0 THEN
						NULL
					ELSE
						$9
				END
			),
			liveness_probe_port_type = (
				CASE
					WHEN $8 = 0 THEN
						NULL
					WHEN $8 IS NULL THEN
						liveness_probe_port_type
					ELSE
						'http'::EXPOSED_PORT_TYPE
				END
			)
		WHERE
			id = $10;
		"#,
		name as _,
		machine_type as _,
		deploy_on_push,
		min_horizontal_scale.map(|v| v as i16),
		max_horizontal_scale.map(|v| v as i16),
		startup_probe.map(|probe| probe.port as i32),
		startup_probe.as_ref().map(|probe| probe.path.as_str()),
		liveness_probe.map(|probe| probe.port as i32),
		liveness_probe.as_ref().map(|probe| probe.path.as_str()),
		deployment_id as _
	)
	.execute(&mut **database)
	.await?;

	// END DEFERRED CONSTRAINT
	query!(
		r#"
		SET CONSTRAINTS ALL IMMEDIATE;
		"#,
	)
	.execute(&mut **database)
	.await?;

	if let Some(config_mounts) = config_mounts {
		query!(
			r#"
			DELETE FROM
				deployment_config_mounts
			WHERE
				deployment_id = $1;
			"#,
			deployment_id as _,
		)
		.execute(&mut **database)
		.await?;

		for (path, file) in &config_mounts {
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
	}

	if let Some(updated_volumes) = volumes {
		let mut current_volumes = query!(
			r#"
				SELECT
					id,
					name,
					deployment_id
					volume_size,
					volume_mount_path
				FROM
					deployment_volume
				WHERE
					deployment_id = $1 AND
					deleted IS NULL;
				"#,
			deployment_id as _,
		)
		.fetch_all(&mut **database)
		.await?
		.into_iter()
		.map(|volume| (volume.name.clone(), volume))
		.collect::<BTreeMap<_, _>>();

		for (name, volume) in updated_volumes {
			if let Some(value) = current_volumes.remove(&name) {
				// The new volume is there in the current volumes. Update it
				let current_size = value.volume_size.as_u128() as u16;
				let new_size = volume.size;

				match new_size.cmp(&current_size) {
					Ordering::Less => {
						// Volume size cannot be reduced
						return Err(ErrorType::ReducedVolumeSize);
					}
					Ordering::Equal => (), // Ignore
					Ordering::Greater => {
						query!(
							r#"
							UPDATE 
								deployment_volume
							SET
								volume_size = $1
							WHERE
								name = $2 AND
								deployment_id = $3;
							"#,
							volume_size as i32,
							name,
							deployment_id as _,
						)
						.execute(&mut **database)
						.await?;
					}
				}
			} else {
				// The new volume is not there in the current volumes. Prevent
				// from adding it
				return Err(ErrorType::CannotAddNewVolume);
			}
		}

		if !current_volumes.is_empty() {
			// Preventing removing number of volume
			return Err(ErrorType::CannotRemoveVolume);
		}
	}

	if let Some(environment_variables) = environment_variables {
		query!(
			r#"
			DELETE FROM
				deployment_environment_variable
			WHERE
				deployment_id = $1;
			"#,
			deployment_id as _,
		)
		.execute(&mut **database)
		.await?;

		for (key, value) in environment_variables {
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
						None
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
						None,
						Some(secret_id.into())
					)
					.execute(&mut **database)
					.await?;
				}
			}
		}
	}

	if let Some(new_min_replica) = min_horizontal_scale {
		if new_min_replica != deployment.min_horizontal_scale.into() {
			for volume in &volumes {
				if todo!("if patr cluster") {
					todo!("stop and start volume usage history");
				};
			}
		}
	}

	let deployment_status = query!(
		r#"
		SELECT
			status
		FROM
			deployment
		WHERE
			id = $1 AND
			status != 'deleted';
		"#,
		deployment_id as _
	)
	.fetch_optional(&mut **database)
	.await?
	.map(|deployment| deployment.status)
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	match deployment_status {
		DeploymentStatus::Stopped | DeploymentStatus::Deleted | DeploymentStatus::Created => {
			// Don't update deployments that are explicitly stopped or deleted
		}
		_ => {
			query!(
				r#"
				UPDATE
					deployment
				SET
					status = $1
				WHERE
					id = $2;
				"#,
				DeploymentStatus::Deploying as _,
				deployment_id as _
			)
			.execute(&mut **database)
			.await?;

			if todo!("Deployment on patr region") {
				todo!("Start and stop deployment usage history")
			}
		}
	}

	AppResponse::builder()
		.body(UpdateDeploymentResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn list_linked_url(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ListLinkedURLPath {
					workspace_id,
					deployment_id,
				},
				query: _,
				headers,
				body,
			},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, ListLinkedURLRequest>,
) -> Result<AppResponse<ListLinkedURLRequest>, ErrorType> {
	info!("Starting: List linked URLs");

	let urls = query!(
		r#"
		SELECT
			id,
			sub_domain,
			domain_id,
			path,
			url_type,
			is_configured,
			deployment_id,
			port,
			static_site_id,
			http_only,
			url,
			permanent_redirect
		FROM
			managed_url
		WHERE
			managed_url.deployment_id = $1 AND
			managed_url.workspace_id = $2 AND
			managed_url.url_type = 'proxy_to_deployment' AND
			managed_url.deleted IS NULL;
		"#,
		deployment_id as _,
		workspace_id as _
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::ResourceDoesNotExist)
	.into_iter()
	.map(|url| {
		WithId::new(
			url.id.into(),
			ManagedUrl {
				sub_domain: url.sub_domain,
				domain_id: url.domain_id.into(),
				path: url.path,
				url_type: match url.url_type {
					DbManagedUrlType::ProxyToDeployment => ManagedUrlType::ProxyDeployment {
						deployment_id: url.deployment_id.unwrap().into(),
						port: url.port.unwrap() as u16,
					},
					DbManagedUrlType::ProxyToStaticSite => ManagedUrlType::ProxyStaticSite {
						static_site_id: url.static_site_id.unwrap().into(),
					},
					DbManagedUrlType::ProxyUrl => ManagedUrlType::ProxyUrl {
						url: url.url.unwrap(),
						http_only: url.http_only.unwrap(),
					},
					DbManagedUrlType::Redirect => ManagedUrlType::Redirect {
						url: url.url.unwrap(),
						permanent_redirect: url.permanent_redirect.unwrap(),
						http_only: url.http_only.unwrap(),
					},
				},
				is_configured: url.is_configured,
			},
		)
	})
	.collect();

	AppResponse::builder()
		.body(ListLinkedURLResponse { urls })
		.headers(ListLinkedURLResponseHeaders {
			total_count: todo!(),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn get_deployment_metric(
	AuthenticatedAppRequest {
		request: ProcessedApiRequest {
			path,
			query: _,
			headers,
			body,
		},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, GetDeploymentMetricRequest>,
) -> Result<AppResponse<GetDeploymentMetricRequest>, ErrorType> {
	info!("Starting: Get deployment metrics");

	// LOGIC

	AppResponse::builder()
		.body(GetDeploymentMetricResponse { metrics: todo!() })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
