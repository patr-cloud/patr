use models::{
	api::workspace::{deployment::*, runner::StreamRunnerDataForWorkspaceServerMsg},
	utils::StringifiedU16,
};

use crate::prelude::*;

/// Loads what a runner should know about a deployment now: its current state if
/// it's live, in the workspace and on this runner, or that it's gone otherwise.
/// The status is the public one, the same one the API serves.
pub(super) async fn load_event(
	connection: &mut DatabaseConnection,
	workspace_id: Uuid,
	runner_id: Uuid,
	deployment_id: Uuid,
) -> Result<StreamRunnerDataForWorkspaceServerMsg, ErrorType> {
	let Some(row) = query!(
		r#"
		SELECT
			deployment.id,
			deployment.name,
			registry,
			repository_id,
			image_name,
			image_tag,
			CASE
				WHEN NOT IS_RUNNER_ONLINE(runner.is_connected, runner.last_seen) THEN 'unreachable'
				ELSE deployment.status
			END AS "status!",
			should_run,
			deployment.runner,
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
		INNER JOIN
			runner
		ON
			runner.id = deployment.runner
		WHERE
			deployment.id = $1 AND
			deployment.workspace_id = $2 AND
			deployment.runner = $3 AND
			deployment.deleted IS NULL;
		"#,
		deployment_id,
		workspace_id,
		runner_id,
	)
	.fetch_optional(&mut *connection)
	.await?
	else {
		return Ok(StreamRunnerDataForWorkspaceServerMsg::DeploymentDeleted { id: deployment_id });
	};

	let ports = query!(
		r#"
		SELECT
			port,
			port_type
		FROM
			deployment_exposed_port
		WHERE
			deployment_id = $1;
		"#,
		deployment_id
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| (StringifiedU16::new(row.port as u16), row.port_type))
	.collect();

	let environment_variables = query!(
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
		deployment_id
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.filter_map(|env| match (env.value, env.secret_id) {
		(Some(value), None) => Some((env.name, EnvironmentVariableValue::String(value))),
		(None, Some(secret_id)) => Some((
			env.name,
			EnvironmentVariableValue::Secret {
				from_secret: secret_id,
			},
		)),
		_ => None,
	})
	.collect();

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
		deployment_id
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|mount| (mount.path, mount.file.into()))
	.collect();

	let volumes = query!(
		r#"
		SELECT
			path
		FROM
			deployment_volume
		WHERE
			deployment_id = $1;
		"#,
		deployment_id,
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| (row.path, VolumeConfig {}))
	.collect();

	Ok(StreamRunnerDataForWorkspaceServerMsg::DeploymentUpdated {
		deployment: WithId::new(
			row.id,
			Deployment {
				name: row.name,
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
				image_tag: row.image_tag,
				status: row.status,
				should_run: row.should_run,
				runner: row.runner,
				machine_type: row.machine_type,
				current_live_digest: row.current_live_digest,
			},
		),
		running_details: DeploymentRunningDetails {
			deploy_on_push: row.deploy_on_push,
			min_horizontal_scale: row.min_horizontal_scale as u16,
			max_horizontal_scale: row.max_horizontal_scale as u16,
			ports,
			environment_variables,
			startup_probe: row.startup_probe_port.zip(row.startup_probe_path).map(
				|(port, path)| DeploymentProbe {
					port: port as u16,
					path,
				},
			),
			liveness_probe: row.liveness_probe_port.zip(row.liveness_probe_path).map(
				|(port, path)| DeploymentProbe {
					port: port as u16,
					path,
				},
			),
			config_mounts,
			volumes,
		},
	})
}
