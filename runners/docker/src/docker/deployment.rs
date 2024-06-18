use std::collections::HashMap;

use bollard::{container::*, Docker};
use models::api::workspace::deployment::*;

use crate::{config::RunnerSettings, prelude::*};

pub async fn created(
	settings: &RunnerSettings,
	docker: Docker,
	deployment: WithId<Deployment>,
	running_details: DeploymentRunningDetails,
) -> Result<(), String> {
	docker
		.create_container(
			Some(CreateContainerOptions {
				name: deployment.data.name.clone(),
				..Default::default()
			}),
			Config {
				image: Some(format!(
					"{}/{}:{}",
					deployment.data.registry.registry_url(),
					deployment.data.registry.image_name().unwrap(),
					deployment.data.image_tag
				)),
				env: Some(
					running_details
						.environment_variables
						.iter()
						.filter_map(|(k, v)| match v {
							EnvironmentVariableValue::String(value) => {
								Some(format!("{}={}", k, value))
							}
							EnvironmentVariableValue::Secret { from_secret: _ } => None,
						})
						.collect(),
				),
				labels: Some(HashMap::from([
					(
						"patr.workspace_id".to_string(),
						settings.workspace_id.to_string(),
					),
					("patr.runner_id".to_string(), settings.runner_id.to_string()),
					("patr.deployment_id".to_string(), deployment.id.to_string()),
				])),
				..Default::default()
			},
		)
		.await
		.expect("Failed to create container");
	docker
		.start_container(&deployment.data.name, None::<StartContainerOptions<String>>)
		.await
		.expect("Failed to start container");
	Ok(())
}

pub async fn deleted(
	_settings: &RunnerSettings,
	docker: Docker,
	deployment_id: Uuid,
) -> Result<(), String> {
	let containers = docker
		.list_containers(Some(ListContainersOptions {
			filters: HashMap::from([(
				"label".to_string(),
				vec![format!("patr.deployment_id={}", deployment_id)],
			)]),
			all: true,
			size: false,
			limit: None,
		}))
		.await
		.expect("Failed to list containers");
	let container = containers.first().expect("No container found");
	docker
		.remove_container(
			container.id.as_deref().unwrap(),
			Some(RemoveContainerOptions {
				v: true,
				force: true,
				link: false,
			}),
		)
		.await
		.expect("Failed to remove container");
	Ok(())
}
