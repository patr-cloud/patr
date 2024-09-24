use http::StatusCode;
use models::api::workspace::{deployment::*, runner::StreamRunnerDataForWorkspaceServerMsg};

use crate::prelude::*;

/// The handler to create a deployment. This will create a new deployment, and
/// return the ID of the deployment.
pub async fn create_deployment(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: CreateDeploymentPath { workspace_id: _ },
				query: (),
				headers:
					CreateDeploymentRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body:
					CreateDeploymentRequestProcessed {
						name,
						registry,
						image_tag,
						runner: _,
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
		runner_changes_sender,
		config: _,
	}: AppRequest<'_, CreateDeploymentRequest>,
) -> Result<AppResponse<CreateDeploymentRequest>, ErrorType> {
	trace!("Creating deployment: {}", name);

	let deployment_id = Uuid::new_v4();

	let status = if deploy_on_create {
		DeploymentStatus::Running
	} else {
		DeploymentStatus::Created
	};
	query(
		r#"
		INSERT INTO
			deployment(
				id,
				name,
				registry,
				image_name,
				image_tag,
				status,
				machine_type,
				min_horizontal_scale,
				max_horizontal_scale,
				deploy_on_push,
				startup_probe_port,
				startup_probe_path,
				startup_probe_port_type,
				liveness_probe_port,
				liveness_probe_path,
				liveness_probe_port_type,
				current_live_digest,
				deleted
			)
		VALUES
			(
				$1,
				$2,
				$3,
				$4,
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
				$16,
				NULL,
				NULL
			);
		"#,
	)
	.bind(deployment_id)
	.bind(name.to_string())
	.bind(registry.registry_url())
	.bind(registry.image_name())
	.bind(image_tag.to_string())
	.bind(status)
	.bind(machine_type)
	.bind(min_horizontal_scale)
	.bind(max_horizontal_scale)
	.bind(deploy_on_push)
	.bind(startup_probe.as_ref().map(|probe| probe.port))
	.bind(startup_probe.as_ref().map(|probe| probe.path.as_str()))
	.bind(startup_probe.as_ref().map(|_| ExposedPortType::Http))
	.bind(liveness_probe.as_ref().map(|probe| probe.port))
	.bind(liveness_probe.as_ref().map(|probe| probe.path.as_str()))
	.bind(liveness_probe.as_ref().map(|_| ExposedPortType::Http))
	.execute(&mut **database)
	.await?;

	trace!("Created deployment with ID: {}", deployment_id);

	for (port, port_type) in &ports {
		query(
			r#"
			INSERT INTO
				deployment_exposed_port(
					deployment_id,
					port,
					port_type
				)
			VALUES
				(
					$1,
					$2,
					$3
				);
			"#,
		)
		.bind(deployment_id)
		.bind(port.value())
		.bind(port_type)
		.execute(&mut **database)
		.await?;
	}

	trace!("Inserted exposed ports for deployment");

	for (name, value) in &environment_variables {
		query(
			r#"
			INSERT INTO
				deployment_environment_variable(
					deployment_id,
					name,
					value,
					secret_id
				)
			VALUES
				(
					$1,
					$2,
					$3,
					$4
				);
			"#,
		)
		.bind(deployment_id)
		.bind(name)
		.bind(value.value())
		.bind(value.secret_id())
		.execute(&mut **database)
		.await?;
	}

	trace!("Inserted environment variables for deployment");

	for (path, file) in &config_mounts {
		query(
			r#"
			INSERT INTO
				deployment_config_mounts(
					deployment_id,
					path,
					file
				)
			VALUES
				(
					$1,
					$2,
					$3
				);
			"#,
		)
		.bind(deployment_id)
		.bind(path)
		.bind(file.to_vec())
		.execute(&mut **database)
		.await?;
	}

	trace!("Inserted config mounts for deployment");

	for (volume_id, mount_path) in &volumes {
		query(
			r#"
			INSERT INTO 
				deployment_volume_mount(
					deployment_id,
					volume_id,
					volume_mount_path
				)
			VALUES
				(
					$1,
					$2,
					$3
				);
			"#,
		)
		.bind(deployment_id)
		.bind(volume_id)
		.bind(mount_path)
		.execute(&mut **database)
		.await?;
	}

	trace!("Inserted volume mounts for deployment");

	runner_changes_sender
		.send(StreamRunnerDataForWorkspaceServerMsg::DeploymentCreated {
			deployment: WithId::new(
				deployment_id,
				Deployment {
					name: name.to_string(),
					registry,
					image_tag: image_tag.to_string(),
					status,
					runner: Uuid::nil(),
					machine_type,
					current_live_digest: None,
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
		.expect("Failed to send deployment created message");

	AppResponse::builder()
		.body(CreateDeploymentResponse {
			id: WithId::from(deployment_id),
		})
		.headers(())
		.status_code(StatusCode::CREATED)
		.build()
		.into_result()
}
