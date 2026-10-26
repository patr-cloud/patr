use std::collections::{BTreeMap, BTreeSet};

use axum::http::StatusCode;
#[cfg(feature = "cloud")]
use cloudflare::{
	endpoints::workerskv::write_key,
	framework::{
		Environment,
		auth::Credentials,
		client::{ClientConfig, async_api::Client as CloudflareClient},
	},
};
#[cfg(feature = "cloud")]
use models::cloudflare::kv::*;
use models::{api::workspace::deployment::*, rbac::ResourceType, utils::StringifiedU16};
use rustis::commands::GenericCommands as _;
use time::OffsetDateTime;

use crate::{
	models::permissions,
	prelude::*,
	redis_publisher::{self, DirtyResource},
};

/// Update deployment details. This endpoint is used to update the deployment
/// details. The deployment details that can be updated are the name, machine
/// type, deploy on push, min horizontal scale, max horizontal scale, ports,
/// environment variables, startup probe, liveness probe, config mounts, and
/// volumes.
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
						image_tag,
						machine_type,
						runner,
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
					},
			},
		database,
		redis,
		client_ip: _,
		actor_data,
		state,
	}: AuthenticatedAppRequest<'_, UpdateDeploymentRequest>,
) -> Result<AppResponse<UpdateDeploymentRequest>, ErrorType> {
	info!("Updating deployment: {}", deployment_id);

	// Horizontal scale must be at least 1 (a deployment can't run zero replicas).
	if min_horizontal_scale == 0 || max_horizontal_scale == 0 {
		return Err(ErrorType::WrongParameters);
	}

	// Volumes are node-local, so a deployment that has any can only run one
	// replica — the same rule Render, Railway and CapRover apply.
	if !volumes.is_empty() && (min_horizontal_scale > 1 || max_horizontal_scale > 1) {
		return Err(ErrorType::VolumesRequireSingleReplica);
	}

	let now = OffsetDateTime::now_utc();

	let existing = query!(
		r#"
		SELECT
			registry,
			repository_id,
			image_tag,
			runner
		FROM
			deployment
		WHERE
			id = $1 AND
			deleted IS NULL;
		"#,
		deployment_id,
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	let old_image_tag = existing.image_tag;
	let repository_id = existing.repository_id;
	let is_patr_registry = existing.registry == PatrRegistry.to_string();
	// Cloned so the requested tag is still available after the UPDATE below.
	let new_image_tag = image_tag.to_string();

	// BEGIN DEFERRED CONSTRAINT
	query!(
		r#"
		SET CONSTRAINTS ALL DEFERRED;
		"#,
	)
	.execute(&mut **database)
	.await?;

	query!(
		r#"
		DELETE FROM
			deployment_exposed_port
		WHERE
			deployment_id = $1;
		"#,
		deployment_id,
	)
	.execute(&mut **database)
	.await?;

	let ports = query!(
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
			port_type;
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
	.collect::<BTreeMap<_, _>>();

	// Updating deployment details
	let updated = query!(
		r#"
		UPDATE
			deployment
		SET
			name = $1,
			image_tag = $2,
			machine_type = $3,
			deploy_on_push = $4,
			runner = $5,
			observed_status = CASE
				WHEN runner = $5 THEN observed_status
				ELSE NULL
			END,
			min_horizontal_scale = $6,
			max_horizontal_scale = $7,
			startup_probe_port = $8,
			startup_probe_path = $9,
			startup_probe_port_type = $10,
			liveness_probe_port = $11,
			liveness_probe_path = $12,
			liveness_probe_port_type = $13
		WHERE
			id = $14
		RETURNING
			runner,
			should_run;
		"#,
		name as _,
		image_tag as _,
		machine_type,
		deploy_on_push,
		runner,
		min_horizontal_scale as i16,
		max_horizontal_scale as i16,
		startup_probe.as_ref().map(|probe| probe.port as i32),
		startup_probe.as_ref().map(|probe| probe.path.as_str()),
		startup_probe.as_ref().map(|_| ExposedPortType::Http),
		liveness_probe.as_ref().map(|probe| probe.port as i32),
		liveness_probe.as_ref().map(|probe| probe.path.as_str()),
		liveness_probe.as_ref().map(|_| ExposedPortType::Http),
		deployment_id,
	)
	.fetch_one(&mut **database)
	.await
	.map_err(|err| match err {
		sqlx::Error::Database(err)
			if err.constraint() == Some("deployment_fk_runner_workspace_id") =>
		{
			ErrorType::WrongParameters
		}
		err => ErrorType::server_error(err),
	})?;

	// END DEFERRED CONSTRAINT
	query!(
		r#"
		SET CONSTRAINTS ALL IMMEDIATE;
		"#,
	)
	.execute(&mut **database)
	.await?;

	// Invalidate the cached runner-for-deployment lookup, or the loki and mimir
	// push proxies keep rejecting the new runner's pushes after a move
	redis
		.del(redis::keys::runner_id_for_deployment(&deployment_id))
		.await?;

	// If the image tag actually changed, re-resolve current_live_digest so the
	// deployment redeploys the new image. The runner's docker executor pins to
	// current_live_digest over the tag, so updating image_tag alone would keep
	// running the old image. Gated on a real change because the frontend sends
	// image_tag on every update.
	if new_image_tag != old_image_tag {
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
					repository_id,
					new_image_tag.as_str(),
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
					deployment_id,
					digest,
					repository_id,
					now,
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
			deployment_id,
			new_digest,
		)
		.execute(&mut **database)
		.await?;
	}

	// Referencing a secret hands its value to the deployment, so a newly
	// referenced secret takes `Secret::View`. Secrets the deployment already
	// references are left alone, so an edit to something else isn't refused.
	// The FK on the insert below keeps the secret in this workspace.
	let existing_secrets = query!(
		r#"
		SELECT
			secret_id AS "secret_id!"
		FROM
			deployment_environment_variable
		WHERE
			deployment_id = $1 AND
			secret_id IS NOT NULL;
		"#,
		deployment_id,
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|row| row.secret_id)
	.collect::<BTreeSet<_>>();

	let view_secret_permission =
		permissions::get_permission_id(database, Permission::Secret(SecretPermission::View)).await;
	if environment_variables
		.values()
		.filter_map(EnvironmentVariableValue::secret_id)
		.filter(|secret_id| !existing_secrets.contains(secret_id))
		.any(|secret_id| {
			!actor_data.has_permission_on_resource(workspace_id, secret_id, view_secret_permission)
		}) {
		return Err(ErrorType::Unauthorized);
	}

	query!(
		r#"
		DELETE FROM
			deployment_environment_variable
		WHERE
			deployment_id = $1;
		"#,
		deployment_id,
	)
	.execute(&mut **database)
	.await?;

	query!(
		r#"
		INSERT INTO
			deployment_environment_variable(
				deployment_id,
				workspace_id,
				name,
				value,
				secret_id
			)
		SELECT
			*
		FROM
			UNNEST(
				$1::UUID[],
				$2::UUID[],
				$3::TEXT[],
				$4::TEXT[],
				$5::UUID[]
			);
		"#,
		&environment_variables
			.iter()
			.map(|_| deployment_id.into())
			.collect::<Vec<sqlx::types::Uuid>>(),
		&environment_variables
			.iter()
			.map(|_| workspace_id.into())
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
	.await
	.map_err(|err| match err {
		sqlx::Error::Database(err)
			if err.constraint() ==
				Some("deployment_environment_variable_fk_secret_id_workspace_id") =>
		{
			ErrorType::ResourceDoesNotExist
		}
		err => ErrorType::server_error(err),
	})?;

	query!(
		r#"
		DELETE FROM
			deployment_config_mounts
		WHERE
			deployment_id = $1;
		"#,
		deployment_id,
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

	query!(
		r#"
		DELETE FROM
			deployment_volume
		WHERE
			deployment_id = $1;
		"#,
		deployment_id,
	)
	.execute(&mut **database)
	.await?;

	query!(
		r#"
		INSERT INTO
			deployment_volume(
				deployment_id,
				path
			)
		SELECT
			*
		FROM
			UNNEST(
				$1::UUID[],
				$2::TEXT[]
			);
		"#,
		&volumes
			.iter()
			.map(|_| deployment_id.into())
			.collect::<Vec<_>>(),
		&volumes.keys().cloned().collect::<Vec<_>>(),
	)
	.execute(&mut **database)
	.await
	.map_err(|err| match err {
		// The path CHECK constraint is the validation for volume paths.
		sqlx::Error::Database(err) if err.is_check_violation() => ErrorType::WrongParameters,
		err => ErrorType::server_error(err),
	})?;

	cfg_if! {
		if #[cfg(feature = "cloud")] {
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
				body: write_key::WriteKeyBody::Value(serde_json::to_vec(
					&InternalKVData::Deployment {
						ports: ports.keys().map(|port| port.value()).collect(),
						runner_id: updated.runner,
						should_run: updated.should_run,
					},
				)?),
			})
			.await?;
		} else {
			let _ = (&state, ports); // Avoid unused variable warning
		}
	}

	// On a move, the old runner is told too, so it drops the deployment and its
	// managed URLs.
	let runners = BTreeSet::from([existing.runner, updated.runner]);
	for runner_id in &runners {
		redis_publisher::mark_dirty(
			database,
			DirtyResource {
				workspace_id,
				runner_id: *runner_id,
				resource_type: ResourceType::Deployment,
				resource_id: deployment_id,
			},
		)
		.await?;
	}

	if existing.runner != updated.runner {
		let managed_urls = query!(
			r#"
			SELECT
				managed_url.id,
				managed_url.sub_domain,
				CONCAT(
					workspace_domain.name,
					'.',
					workspace_domain.tld
				) AS "domain!"
			FROM
				managed_url
			INNER JOIN
				workspace_domain
			ON
				managed_url.domain_id = workspace_domain.id
			WHERE
				managed_url.deployment_id = $1 AND
				managed_url.deleted IS NULL;
			"#,
			deployment_id,
		)
		.fetch_all(&mut **database)
		.await?;

		for managed_url in &managed_urls {
			for runner_id in &runners {
				redis_publisher::mark_dirty(
					database,
					DirtyResource {
						workspace_id,
						runner_id: *runner_id,
						resource_type: ResourceType::ManagedURL,
						resource_id: managed_url.id,
					},
				)
				.await?;
			}
		}

		// The ingress KV routes each managed URL to its deployment's runner
		cfg_if! {
			if #[cfg(feature = "cloud")] {
				for fqdn in managed_urls
					.iter()
					.map(|managed_url| format!("{}.{}", managed_url.sub_domain, managed_url.domain))
					.collect::<BTreeSet<_>>()
				{
					utils::cloudflare::sync_ingress_kv_for_fqdn(&fqdn, database, &state.config)
						.await?;
				}
			}
		}
	}

	AppResponse::builder()
		.body(UpdateDeploymentResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
