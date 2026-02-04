use std::collections::HashMap;

use bollard::{
	Docker,
	query_parameters::UpdateServiceOptionsBuilder,
	service::{
		EndpointPortConfig,
		EndpointPortConfigProtocolEnum,
		EndpointPortConfigPublishModeEnum,
		EndpointSpec,
		NetworkAttachmentConfig,
		ServiceSpec,
		ServiceSpecMode,
		ServiceSpecModeReplicated,
		TaskSpec,
		TaskSpecContainerSpec,
	},
};

use crate::prelude::*;

/// Ensure the ingress service is running, updating the configs with the
/// latest deployment configs, if required.
///
/// This will first check if the ingress service exists, and create it if it
/// does not. Then, it will update the deployment configs for all deployments
/// and then mount all the configs into the ingress service and reload ingress
/// to pick up the new configs.
pub async fn update_ingress_configs(
	docker: &Docker,
	settings: &DockerSettings,
) -> Result<(), RunnerError> {
	let service_spec = ServiceSpec {
		name: Some(String::from(constants::INGRESS_SERVICE_NAME)),
		labels: Some(HashMap::from([(
			String::from("managed-by"),
			String::from("patr"),
		)])),
		task_template: Some(TaskSpec {
			plugin_spec: None,
			container_spec: Some(TaskSpecContainerSpec {
				image: Some(String::from("caddy:latest")),
				labels: Some(HashMap::from([(
					String::from("managed-by"),
					String::from("patr"),
				)])),
				..Default::default()
			}),
			..Default::default()
		}),
		mode: Some(ServiceSpecMode {
			replicated: Some(ServiceSpecModeReplicated { replicas: Some(1) }),
			..Default::default()
		}),
		endpoint_spec: Some(EndpointSpec {
			mode: None,
			ports: Some(vec![
				EndpointPortConfig {
					name: None,
					protocol: Some(EndpointPortConfigProtocolEnum::TCP),
					target_port: Some(80),
					published_port: Some(settings.ingress_http_listen_port.into()),
					publish_mode: Some(EndpointPortConfigPublishModeEnum::INGRESS),
				},
				EndpointPortConfig {
					name: None,
					protocol: Some(EndpointPortConfigProtocolEnum::TCP),
					target_port: Some(443),
					published_port: Some(settings.ingress_https_listen_port.into()),
					publish_mode: Some(EndpointPortConfigPublishModeEnum::INGRESS),
				},
			]),
		}),
		networks: Some(vec![
			NetworkAttachmentConfig {
				target: Some(String::from(constants::INGRESS_NETWORK_NAME)),
				aliases: Some(vec![String::from("patr-ingress"), String::from("ingress")]),
				driver_opts: None,
			},
			NetworkAttachmentConfig {
				target: Some(String::from("ingress")),
				aliases: Some(vec![String::from("patr-ingress"), String::from("ingress")]),
				driver_opts: None,
			},
		]),
		..Default::default()
	};

	let ingress = docker
		.inspect_service(constants::INGRESS_SERVICE_NAME, None)
		.await
		.ok();

	if let Some(version) = ingress
		.and_then(|ingress| ingress.version)
		.and_then(|version| version.index)
	{
		docker
			.update_service(
				constants::INGRESS_SERVICE_NAME,
				service_spec,
				UpdateServiceOptionsBuilder::new()
					.version(version as i32)
					.build(),
				None,
			)
			.await
			.map_err(RunnerError::host)?;
	} else {
		docker
			.create_service(service_spec, None)
			.await
			.map_err(RunnerError::host)?;
	}

	Ok(())
}
