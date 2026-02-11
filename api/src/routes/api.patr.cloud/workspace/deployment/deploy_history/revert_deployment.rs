use std::collections::BTreeMap;

use axum::http::StatusCode;
use models::{
	api::workspace::{
		deployment::{deploy_history::*, *},
		runner::StreamRunnerDataForWorkspaceServerMsg,
	},
	utils::{Base64String, StringifiedU16},
};
use rustis::commands::PubSubCommands;

use crate::prelude::*;
/// Revert a deployment to a previous deployment history, using the image
/// digest.
pub async fn revert_deployment(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path:
					RevertDeploymentPath {
						workspace_id,
						deployment_id,
						image_digest,
					},
				query: (),
				headers:
					RevertDeploymentRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: RevertDeploymentRequestProcessed,
			},
		database,
		redis,
		client_ip: _,
		user_data: _,
		state: _,
	}: AuthenticatedAppRequest<'_, RevertDeploymentRequest>,
) -> Result<AppResponse<RevertDeploymentRequest>, ErrorType> {
	info!(
		"Reverting deployment `{}` to image: {}",
		deployment_id, image_digest
	);

	// Revert the deployment to the specified image digest if the deployment exists
	let exists = query!(
		r#"
		SELECT
			*
        FROM
            deployment_deploy_history
        WHERE
            deployment_id = $1 AND
            image_digest = $2;
		"#,
		deployment_id as _,
		image_digest
	)
	.fetch_optional(&mut **database)
	.await?
	.is_some();

	if !exists {
		info!(
			"Deployment history with deployment ID `{}` and image digest `{}` does not exist",
			deployment_id, image_digest
		);
		return Err(ErrorType::ResourceDoesNotExist);
	}

	query!(
		r#"
        UPDATE
            deployment
        SET
            current_live_digest = $1
        WHERE
            id = $2;
        "#,
		image_digest,
		deployment_id as _
	)
	.execute(&mut **database)
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
		let value = env.value.map(EnvironmentVariableValue::String);

		let secret_id = env
			.secret_id
			.map(|from_secret| EnvironmentVariableValue::Secret { from_secret });

		let value = match (value, secret_id) {
			(Some(value), None) => Some(value),
			(None, Some(secret)) => Some(secret),
			_ => None,
		}
		.ok_or(ErrorType::server_error(
			"corrupted deployment, cannot find environment variable value",
		))?;

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
		.body(RevertDeploymentResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
