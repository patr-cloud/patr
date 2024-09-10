use models::api::workspace::deployment::*;
use time::OffsetDateTime;

use crate::{
	app::{AppRequest, ProcessedApiRequest},
	prelude::*,
};

pub async fn create_deployment(
	request: AppRequest<'_, CreateDeploymentRequest>,
) -> Result<AppResponse<CreateDeploymentRequest>, ErrorType> {
	let AppRequest {
		database,
		request:
			ProcessedApiRequest {
				path: CreateDeploymentPath { workspace_id: _ },
				query: _,
				headers: _,
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
	} = request;

	let now = OffsetDateTime::now_utc();

	let deployment_id = Uuid::new_v4();

	let created_deployment_id = query(
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
			) VALUES (
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
			)
		"#,
	)
	.bind(deployment_id.to_string())
	.bind(name.to_string())
	.bind(registry.registry_url())
	.bind(registry.image_name())
	.bind(image_tag)
	.bind(
		(if deploy_on_create {
			DeploymentStatus::Running
		} else {
			DeploymentStatus::Created
		})
		.to_string(),
	)
	.bind(machine_type.to_string())
	.bind(min_horizontal_scale)
	.bind(max_horizontal_scale)
	.bind(deploy_on_push)
	.bind(startup_probe.as_ref().map(|probe| probe.port as i32))
	.bind(startup_probe.as_ref().map(|probe| probe.path.as_str()))
	.bind(
		startup_probe
			.as_ref()
			.map(|_| ExposedPortType::Http.to_string()),
	)
	.bind(liveness_probe.as_ref().map(|probe| probe.port as i32))
	.bind(liveness_probe.as_ref().map(|probe| probe.path.as_str()))
	.bind(
		liveness_probe
			.as_ref()
			.map(|_| ExposedPortType::Http.to_string()),
	)
	.fetch_one(&mut **database)
	.await;

	trace!("Created deployment with ID: {}", deployment_id);

	// TODO: Find a way to do this using async iterator
	&ports.iter().for_each(|(port, port_type)| {
		query(
			r#"
			INSERT INTO deployment_exposed_port(
				deployment_id,
				port,
				port_type
			)
			VALUES (
				$1,
				$2,
				$3
			);
			"#,
		)
		.bind(deployment_id.to_string())
		.bind(port.value() as i32)
		.bind(port_type.to_string())
		.execute(&mut **database);
	});

	trace!("Inserted exposed ports for deployment");

	&environment_variables.iter().for_each(|(name, value)| {
		query(
			r#"
		INSERT INTO deployment_environment_variable(
			deployment_id,
			name,
			value,
		)
		VALUES (
			$1,
			$2,
			$3,
		);
		"#,
		)
		.bind(deployment_id.to_string())
		.bind(name.to_string())
		.bind(value.value().unwrap_or(&"".to_string()))
		.execute(&mut **database);
	});

	&config_mounts.iter().for_each(|(path, file)| {
		query(
			r#"
			INSERT INTO deployment_config_mounts(
				deployment_id,
				path,
				file
			)
			VALUES (
				$1,
				$2,
				$3
			);
		"#,
		)
		.bind(deployment_id.to_string())
		.bind(path.to_string())
		.bind(file.to_vec())
		.execute(&mut **database);
	});

	trace!("Inserted environment variables for deployment");

	Err(ErrorType::InternalServerError)
}
