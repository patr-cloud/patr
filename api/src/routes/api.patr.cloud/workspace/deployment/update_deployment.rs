use std::collections::BTreeMap;

use axum::http::StatusCode;
use cloudflare::{
	endpoints::workerskv::write_key,
	framework::{
		Environment,
		auth::Credentials,
		client::{ClientConfig, async_api::Client as CloudflareClient},
	},
};
use models::{
	api::workspace::{deployment::*, runner::StreamRunnerDataForWorkspaceServerMsg},
	cloudflare::kv::*,
	utils::{Base64String, StringifiedU16},
};
use rustis::commands::PubSubCommands;
use time::OffsetDateTime;

use crate::prelude::*;

/// Update deployment details. This endpoint is used to update the deployment
/// details. The deployment details that can be updated are the name, machine
/// type, deploy on push, min horizontal scale, max horizontal scale, ports,
/// environment variables, startup probe, liveness probe, config mounts, and
/// volumes. At least one of the values must be updated.
pub async fn update_deployment(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: UpdateDeploymentPath {
					workspace_id,
					deployment_id,
				},
				query: (),
				headers:
					UpdateDeploymentRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body:
					UpdateDeploymentRequestProcessed {
						name,
						machine_type,
						deploy_on_push,
						runner,
						min_horizontal_scale,
						max_horizontal_scale,
						ports,
						environment_variables,
						startup_probe,
						liveness_probe,
						config_mounts,
						volumes,
						image_tag,
					},
			},
		database,
		redis,
		client_ip: _,
		user_data: _,
		state,
	}: AuthenticatedAppRequest<'_, UpdateDeploymentRequest>,
) -> Result<AppResponse<UpdateDeploymentRequest>, ErrorType> {
	info!("Updating deployment: {}", deployment_id);

	let now = OffsetDateTime::now_utc();

	// Validate if at least value is to be updated
	if name
		.as_ref()
		.map(|_| 0)
		.or(machine_type.as_ref().map(|_| 0))
		.or(deploy_on_push.as_ref().map(|_| 0))
		.or(runner.as_ref().map(|_| 0))
		.or(min_horizontal_scale.as_ref().map(|_| 0))
		.or(max_horizontal_scale.as_ref().map(|_| 0))
		.or(ports.as_ref().map(|_| 0))
		.or(environment_variables.as_ref().map(|_| 0))
		.or(startup_probe.as_ref().map(|_| 0))
		.or(liveness_probe.as_ref().map(|_| 0))
		.or(config_mounts.as_ref().map(|_| 0))
		.or(volumes.as_ref().map(|_| 0))
		.or(image_tag.as_ref().map(|_| 0))
		.is_none()
	{
		debug!(
			"No parameters provided for updating deployment: {}",
			deployment_id
		);
		return Err(ErrorType::WrongParameters);
	}

	let existing = query!(
		r#"
		SELECT
			registry,
			repository_id AS "repository_id: Uuid",
			image_tag
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

	let old_image_tag = existing.image_tag;
	let repository_id = existing.repository_id;
	let is_patr_registry = existing.registry == PatrRegistry.to_string();
	// Cloned so the requested tag is still available after the UPDATE below.
	let new_image_tag = image_tag.clone();

	// BEGIN DEFERRED CONSTRAINT
	query!(
		r#"
		SET CONSTRAINTS ALL DEFERRED;
		"#,
	)
	.execute(&mut **database)
	.await?;

	let ports = if let Some(ports) = ports {
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

		query!(
			r#"
			INSERT INTO 
				deployment_exposed_port(
					deployment_id,
					port,
					port_type
				)
			SELECT
				*
			FROM
				UNNEST(
					$1::UUID[],
					$2::INTEGER[],
					$3::EXPOSED_PORT_TYPE[]
				)
			RETURNING
				port,
				port_type AS "port_type: ExposedPortType";
			"#,
			&ports
				.iter()
				.map(|_| deployment_id.into())
				.collect::<Vec<_>>(),
			&ports
				.iter()
				.map(|(port, _)| port.value() as i32)
				.collect::<Vec<_>>(),
			&ports
				.iter()
				.map(|(_, port_type)| port_type.to_string())
				.collect::<Vec<String>>() as _,
		)
		.fetch_all(&mut **database)
		.await?
		.into_iter()
		.map(|row| {
			let port = row.port as u16;
			let port_type = row.port_type;

			(StringifiedU16::new(port), port_type)
		})
		.collect::<BTreeMap<_, _>>()
	} else {
		// Fetch existing ports from database
		query!(
			r#"
			SELECT
				port,
				port_type AS "port_type: ExposedPortType"
			FROM
				deployment_exposed_port
			WHERE
				deployment_id = $1;
			"#,
			deployment_id as _
		)
		.fetch_all(&mut **database)
		.await?
		.into_iter()
		.map(|row| {
			let port = row.port as u16;
			let port_type = row.port_type;

			(StringifiedU16::new(port), port_type)
		})
		.collect::<BTreeMap<_, _>>()
	};

	// Updating deployment details
	let runner_id = query!(
		r#"
		UPDATE
			deployment
		SET
			name = COALESCE($1, name),
			image_tag = COALESCE($2, image_tag),
			machine_type = COALESCE($3, machine_type),
			deploy_on_push = COALESCE($4, deploy_on_push),
			runner = COALESCE($5, runner),
			min_horizontal_scale = COALESCE($6, min_horizontal_scale),
			max_horizontal_scale = COALESCE($7, max_horizontal_scale),
			startup_probe_port = (
				CASE
					WHEN $8 = 0 THEN
						NULL
					ELSE
						$8
				END
			),
			startup_probe_path = (
				CASE
					WHEN $8 = 0 THEN
						NULL
					ELSE
						$9
				END
			),
			startup_probe_port_type = (
				CASE
					WHEN $8 = 0 THEN
						NULL
					WHEN $8 IS NULL THEN
						startup_probe_port_type
					ELSE
						'http'::EXPOSED_PORT_TYPE
				END
			),
			liveness_probe_port = (
				CASE
					WHEN $10 = 0 THEN
						NULL
					ELSE
						$10
				END
			),
			liveness_probe_path = (
				CASE
					WHEN $10 = 0 THEN
						NULL
					ELSE
						$11
				END
			),
			liveness_probe_port_type = (
				CASE
					WHEN $10 = 0 THEN
						NULL
					WHEN $10 IS NULL THEN
						liveness_probe_port_type
					ELSE
						'http'::EXPOSED_PORT_TYPE
				END
			)
		WHERE
			id = $12
		RETURNING
			runner AS "runner: Uuid";
		"#,
		name as _,
		image_tag as _,
		machine_type as _,
		deploy_on_push,
		runner as _,
		min_horizontal_scale.map(|v| v as i16),
		max_horizontal_scale.map(|v| v as i16),
		startup_probe.as_ref().map(|probe| probe.port as i32),
		startup_probe.as_ref().map(|probe| probe.path.as_str()),
		liveness_probe.as_ref().map(|probe| probe.port as i32),
		liveness_probe.as_ref().map(|probe| probe.path.as_str()),
		deployment_id as _,
	)
	.fetch_one(&mut **database)
	.await?
	.runner;

	// END DEFERRED CONSTRAINT
	query!(
		r#"
		SET CONSTRAINTS ALL IMMEDIATE;
		"#,
	)
	.execute(&mut **database)
	.await?;

	// If the image tag actually changed, re-resolve current_live_digest so the
	// deployment redeploys the new image. The runner's docker executor pins to
	// current_live_digest over the tag, so updating image_tag alone would keep
	// running the old image. Gated on a real change because the frontend sends
	// image_tag on every update.
	if let Some(new_tag) = new_image_tag
		.as_deref()
		.filter(|&new| new != old_image_tag.as_str())
	{
		let new_digest = if is_patr_registry {
			let digest = if let Some(repository_id) = repository_id {
				query!(
					r#"
					SELECT
						manifest_digest
					FROM
						container_registry_repository_tag
					WHERE
						repository_id = $1 AND
						name = $2;
					"#,
					repository_id as _,
					new_tag,
				)
				.fetch_optional(&mut **database)
				.await?
				.map(|row| row.manifest_digest)
			} else {
				None
			};

			// Record the deploy in history, mirroring start_deployment.
			if let Some(digest) = &digest {
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
				.await?;
			}

			digest
		} else {
			// External registries have no resolvable digest; the executor pulls
			// by tag directly.
			None
		};

		// Overwrite (not COALESCE): a resolvable tag pins the new digest; an
		// unresolvable one clears it so the runner pulls by the new tag and
		// surfaces a bad tag as a pull failure.
		query!(
			r#"
			UPDATE
				deployment
			SET
				current_live_digest = $2
			WHERE
				id = $1;
			"#,
			deployment_id as _,
			new_digest as _,
		)
		.execute(&mut **database)
		.await?;
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

		query!(
			r#"
			INSERT INTO 
				deployment_environment_variable(
					deployment_id,
					name,
					value,
					secret_id
				)
			SELECT
				*
			FROM
				UNNEST(
					$1::UUID[],
					$2::TEXT[],
					$3::TEXT[],
					$4::UUID[]
				);
			"#,
			&environment_variables
				.iter()
				.map(|_| deployment_id.into())
				.collect::<Vec<sqlx::types::Uuid>>(),
			&environment_variables
				.iter()
				.map(|(name, _)| name.clone())
				.collect::<Vec<_>>(),
			&environment_variables
				.iter()
				.map(|(_, value)| value.value().cloned())
				.collect::<Vec<Option<String>>>() as _,
			&environment_variables
				.iter()
				.map(|(_, value)| value.secret_id().map(Into::into))
				.collect::<Vec<Option<sqlx::types::Uuid>>>() as _,
		)
		.execute(&mut **database)
		.await?;
	}

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

		query!(
			r#"
			INSERT INTO 
				deployment_config_mounts(
					deployment_id,
					path,
					file
				)
			SELECT
				*
			FROM
				UNNEST(
					$1::UUID[],
					$2::TEXT[],
					$3::BYTEA[]
				);
			"#,
			&config_mounts
				.iter()
				.map(|_| deployment_id.into())
				.collect::<Vec<_>>(),
			&config_mounts
				.iter()
				.map(|(path, _)| path.clone())
				.collect::<Vec<_>>(),
			&config_mounts
				.iter()
				.map(|(_, file)| file.to_vec())
				.collect::<Vec<_>>(),
		)
		.execute(&mut **database)
		.await?;
	}

	if let Some(updated_volumes) = &volumes {
		query!(
			r#"
			DELETE FROM
				deployment_volume_mount
			WHERE
				deployment_id = $1;
			"#,
			deployment_id as _,
		)
		.execute(&mut **database)
		.await?;

		query!(
			r#"
			INSERT INTO
				deployment_volume_mount(
					deployment_id,
					volume_id,
					volume_mount_path
				)
			SELECT
				*
			FROM
				UNNEST(
					$1::UUID[],
					$2::UUID[],
					$3::TEXT[]
				);
			"#,
			&updated_volumes
				.iter()
				.map(|_| deployment_id.into())
				.collect::<Vec<_>>(),
			&updated_volumes
				.iter()
				.map(|(volume_id, _)| (*volume_id).into())
				.collect::<Vec<_>>(),
			&updated_volumes
				.iter()
				.map(|(_, volume_mount_path)| volume_mount_path.clone())
				.collect::<Vec<_>>(),
		)
		.execute(&mut **database)
		.await
		.map_err(|err| match err {
			sqlx::Error::Database(err) if err.is_unique_violation() => ErrorType::ResourceInUse,
			sqlx::Error::Database(err) if err.is_foreign_key_violation() => {
				ErrorType::ResourceDoesNotExist
			}
			err => ErrorType::server_error(err),
		})?;
	}

	CloudflareClient::new(
		Credentials::UserAuthToken {
			token: state.config.cloudflare.api_key.clone(),
		},
		ClientConfig::default(),
		Environment::Custom(state.config.cloudflare.base_url.clone()),
	)?
	.request(&write_key::WriteKey {
		account_identifier: &state.config.cloudflare.account_id,
		namespace_identifier: &state.config.cloudflare.worker_namespace_id,
		key: &deployment_id.to_string(),
		params: write_key::WriteKeyParams {
			expiration: None,
			expiration_ttl: None,
		},
		body: write_key::WriteKeyBody::Value(serde_json::to_vec(&InternalKVData::Deployment {
			ports: ports.keys().map(|port| port.value()).collect(),
			runner_id,
			status: DeploymentStatus::Deploying,
		})?),
	})
	.await?;

	// TODO Temporary workaround until audit logs and triggers are implemented
	let ports = query!(
		r#"
		SELECT
			port,
			port_type AS "port_type: ExposedPortType"
		FROM
			deployment_exposed_port
		WHERE
			deployment_id = $1;
		"#,
		deployment_id as _
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|row| {
		let port = row.port as u16;
		let port_type = row.port_type;

		Ok((StringifiedU16::new(port), port_type))
	})
	.collect::<Result<BTreeMap<_, _>, ErrorType>>()?;

	let environment_variables = query!(
		r#"
		SELECT
			name,
			value,
			secret_id AS "secret_id: Uuid"
		FROM
			deployment_environment_variable
		WHERE
			deployment_id = $1;
		"#,
		deployment_id as _
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|env| {
		let name = env.name;
		let value = env.value.clone().map(EnvironmentVariableValue::String);

		let secret_id = env
			.secret_id
			.map(|from_secret| EnvironmentVariableValue::Secret { from_secret });

		let value = match (value.clone(), secret_id.clone()) {
			(Some(value), None) => Some(value),
			(None, Some(secret)) => Some(secret),
			_ => None,
		}
		.ok_or_else(|| {
			ErrorType::server_error(format!(
				concat!(
					"corrupted deployment, cannot find environment variable value. ",
					"env name: `{}`, value: {:?}`, secret_id: {:?}, raw_value: {:?}, raw_secret_id: {:?}"
				),
				name, value, secret_id, env.value, env.secret_id
			))
		})?;

		Ok((name, value))
	})
	.collect::<Result<BTreeMap<_, _>, ErrorType>>()?;

	let config_mounts = query!(
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
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|row| {
		let path = row.path;
		let file = Base64String::from(row.file);

		Ok((path, file))
	})
	.collect::<Result<BTreeMap<_, _>, ErrorType>>()?;

	let volumes = query!(
		r#"
		SELECT
			volume_id AS "volume_id: Uuid",
			volume_mount_path
		FROM
			deployment_volume_mount
		WHERE
			deployment_id = $1;
		"#,
		deployment_id as _
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|row| {
		let volume_id = row.volume_id;
		let volume_mount_path = row.volume_mount_path;

		Ok((volume_id, volume_mount_path))
	})
	.collect::<Result<BTreeMap<_, _>, ErrorType>>()?;

	let row = query!(
		r#"
		SELECT
			id AS "id: Uuid",
			name,
			registry,
			image_name,
			image_tag,
			runner AS "runner: Uuid",
			status AS "status: DeploymentStatus",
			repository_id AS "repository_id: Uuid",
			min_horizontal_scale,
			max_horizontal_scale,
			machine_type AS "machine_type: Uuid",
			deploy_on_push,
			startup_probe_port,
			startup_probe_path,
			startup_probe_port_type AS "startup_probe_port_type: Option<ExposedPortType>",
			liveness_probe_port,
			liveness_probe_path,
			liveness_probe_port_type AS "liveness_probe_port_type: Option<ExposedPortType>",
			current_live_digest
		FROM
			deployment
		WHERE
			id = $1 AND
			deleted IS NULL;
		"#,
		deployment_id as _
	)
	.fetch_one(&mut **database)
	.await
	.map_err(|err| match err {
		sqlx::Error::RowNotFound => ErrorType::ResourceDoesNotExist,
		err => err.into(),
	})?;

	let name = row.name;
	let image_tag = row.image_tag;
	let machine_type = row.machine_type;
	let runner = row.runner;

	let deploy_on_push = row.deploy_on_push;
	let min_horizontal_scale = row.min_horizontal_scale as u16;
	let max_horizontal_scale = row.max_horizontal_scale as u16;

	let startup_probe = row
		.startup_probe_port
		.zip(row.startup_probe_path)
		.map(|(port, path)| DeploymentProbe {
			port: port as u16,
			path,
		});

	let liveness_probe =
		row.liveness_probe_port
			.zip(row.liveness_probe_path)
			.map(|(port, path)| DeploymentProbe {
				port: port as u16,
				path,
			});

	redis
		.publish(
			format!("{}/runner/{}/stream", workspace_id, runner),
			serde_json::to_string(&StreamRunnerDataForWorkspaceServerMsg::DeploymentUpdated {
				deployment: WithId::new(
					deployment_id,
					Deployment {
						name: name.to_string(),
						registry: if row.registry == PatrRegistry.to_string() {
							DeploymentRegistry::PatrRegistry {
								registry: PatrRegistry,
								repository_id: row.repository_id.unwrap(),
							}
						} else {
							DeploymentRegistry::ExternalRegistry {
								registry: row.registry,
								image_name: row.image_name.unwrap(),
							}
						},
						image_tag: image_tag.to_string(),
						runner,
						status: DeploymentStatus::Deploying,
						current_live_digest: row.current_live_digest,
						machine_type,
					},
				),
				running_details: DeploymentRunningDetails {
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
			})
			.unwrap(),
		)
		.await?;

	AppResponse::builder()
		.body(UpdateDeploymentResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
