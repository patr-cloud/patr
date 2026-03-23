use std::{collections::HashMap, io::Error as IoError, iter};

use bollard::{
	Docker,
	models::{ConfigSpec, Mount, MountTypeEnum},
	query_parameters::{ListConfigsOptions, UpdateServiceOptionsBuilder},
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
		TaskSpecContainerSpecConfigs,
		TaskSpecContainerSpecFile1,
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
	settings: &RunnerSettings<DockerSettings>,
) -> Result<(), RunnerError> {
	let ingress_service_spec = if settings.data.runner_exposure_type.is_private() {
		get_cloudflare_spec(docker, settings).await?
	} else {
		get_ingress_spec(docker, settings).await?
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
				ingress_service_spec,
				UpdateServiceOptionsBuilder::new()
					.version(version as i32)
					.build(),
				None,
			)
			.await
			.map_err(RunnerError::host)?;
	} else {
		docker
			.create_service(ingress_service_spec, None)
			.await
			.map_err(RunnerError::host)?;
	}

	Ok(())
}

pub fn generate_config_for_deployment(deployment_id: Uuid, port: u16) -> String {
	format!(
		include_str!("../../../assets/runner/Caddyfile.template"),
		deployment_id = deployment_id,
		port = port
	)
}

pub async fn update_ingress_tunnel_token(
	docker: &Docker,
	new_token: String,
) -> Result<(), RunnerError> {
	// Tunnel token config is NOT mounted in any service (cloudflare spec reads
	// it via inspect_config and passes as env var), so we can delete + recreate.
	let existing_config_id = docker
		.list_configs(Some(ListConfigsOptions {
			filters: Some(HashMap::from([(
				String::from("label"),
				vec![format!(
					"patr.deploymentId={}",
					constants::INGRESS_SERVICE_NAME
				)],
			)])),
		}))
		.await
		.map_err(RunnerError::host)?
		.into_iter()
		.find(|config| {
			config
				.spec
				.as_ref()
				.and_then(|spec| spec.name.as_ref())
				.filter(|&name| name == constants::TUNNEL_TOKEN_CONFIG_NAME)
				.is_some()
		})
		.and_then(|config| config.id);

	if let Some(config_id) = existing_config_id {
		docker
			.delete_config(&config_id)
			.await
			.map_err(RunnerError::host)?;
	}

	docker
		.create_config(ConfigSpec {
			name: Some(String::from(constants::TUNNEL_TOKEN_CONFIG_NAME)),
			labels: Some(HashMap::from([
				(String::from("managed-by"), String::from("patr")),
				(
					String::from("patr.deploymentId"),
					String::from(constants::INGRESS_SERVICE_NAME),
				),
			])),
			data: Some(new_token),
			templating: None,
		})
		.await
		.map_err(RunnerError::host)?;

	Ok(())
}

/// Get all ingress configs for deployments.
///
/// This is done by first getting all the deployments that are currently
/// running, then generating the ingress configs for each deployment, writing
/// the configs to it's own Docker Config, and returning a list of all Config
/// IDs for the ingress service to use.
async fn get_deployment_configs(docker: &Docker) -> Result<Vec<(Uuid, String)>, RunnerError> {
	let config_ids = docker
		.list_configs(Some(ListConfigsOptions {
			filters: Some(HashMap::from([(
				String::from("label"),
				vec![String::from("patr.deploymentId")],
			)])),
		}))
		.await
		.map_err(|err| {
			error!("Error listing configs: {:?}", err);
			RunnerError::host(err)
		})?
		.into_iter()
		.filter_map(|config| {
			Some((
				config
					.spec?
					.labels?
					.get("patr.deploymentId")?
					.parse()
					.ok()?,
				config.id?,
			))
		})
		.collect::<Vec<_>>();

	Ok(config_ids)
}

/// Get the service spec for the ingress service, with the latest deployment
/// configs mounted in the service.
async fn get_ingress_spec(
	docker: &Docker,
	settings: &RunnerSettings<DockerSettings>,
) -> Result<ServiceSpec, RunnerError> {
	let config_ids = get_deployment_configs(docker).await?;

	let base_ingress_config = include_str!("../../../assets/runner/Caddyfile.base");
	let base_config = Base64String::from_string(base_ingress_config.to_string());

	let ingress_config_id = crate::utils::update_config(
		docker,
		constants::INGRESS_CONFIG_NAME,
		HashMap::from([(
			String::from("patr.deploymentId"),
			String::from(constants::INGRESS_SERVICE_NAME),
		)]),
		base_config.to_string(),
	)
	.await?;

	Ok(ServiceSpec {
		name: Some(String::from(constants::INGRESS_SERVICE_NAME)),
		labels: Some(HashMap::from([(
			String::from("managed-by"),
			String::from("patr"),
		)])),
		task_template: Some(TaskSpec {
			plugin_spec: None,
			container_spec: Some(TaskSpecContainerSpec {
				image: Some(String::from("caddy:2")),
				labels: Some(HashMap::from([(
					String::from("managed-by"),
					String::from("patr"),
				)])),
				env: Some(vec![format!(
					"ACME_CA_URL={}",
					if cfg!(debug_assertions) {
						"https://acme-staging-v02.api.letsencrypt.org/directory"
					} else {
						"https://acme-v02.api.letsencrypt.org/directory"
					}
				)]),
				configs: Some(
					config_ids
						.into_iter()
						.map(|(deployment_id, config_id)| TaskSpecContainerSpecConfigs {
							file: Some(TaskSpecContainerSpecFile1 {
								name: Some(format!(
									"/etc/caddy/deployments/{}.caddy",
									deployment_id
								)),
								mode: Some(0o444),
								uid: Some("0".to_string()),
								gid: Some("0".to_string()),
							}),
							config_id: Some(config_id),
							config_name: None,
							runtime: None,
						})
						.chain(iter::once(TaskSpecContainerSpecConfigs {
							file: Some(TaskSpecContainerSpecFile1 {
								name: Some(String::from("/etc/caddy/Caddyfile")),
								mode: Some(0o444),
								uid: Some("0".to_string()),
								gid: Some("0".to_string()),
							}),
							config_id: Some(ingress_config_id),
							config_name: None,
							runtime: None,
						}))
						.collect(),
				),
				// Mount a named volume to store the TLS certs, so they persist across service
				// updates and restarts
				mounts: Some(vec![Mount {
					target: Some(String::from("/data")),
					source: Some(String::from(constants::INGRESS_TLS_CERTS_VOLUME_NAME)),
					typ: Some(MountTypeEnum::VOLUME),
					read_only: Some(false),
					..Default::default()
				}]),
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
					published_port: Some(settings.data.ingress_http_listen_port.into()),
					publish_mode: Some(EndpointPortConfigPublishModeEnum::INGRESS),
				},
				EndpointPortConfig {
					name: None,
					protocol: Some(EndpointPortConfigProtocolEnum::TCP),
					target_port: Some(443),
					published_port: Some(settings.data.ingress_https_listen_port.into()),
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
	})
}

/// Get the service spec for the cloudflare tunnel service, with the latest
/// ingress token
async fn get_cloudflare_spec(
	docker: &Docker,
	_: &RunnerSettings<DockerSettings>,
) -> Result<ServiceSpec, RunnerError> {
	let tunnel_token = docker
		.inspect_config(constants::TUNNEL_TOKEN_CONFIG_NAME)
		.await
		.map_err(RunnerError::host)?
		.spec
		.and_then(|spec| spec.data)
		.ok_or(RunnerError::CloudflareTunnelSetupError(IoError::other(
			"could not find cloudflare tunnel token config",
		)))?;

	Ok(ServiceSpec {
		name: Some(String::from(constants::INGRESS_SERVICE_NAME)),
		labels: Some(HashMap::from([(
			String::from("managed-by"),
			String::from("patr"),
		)])),
		task_template: Some(TaskSpec {
			plugin_spec: None,
			container_spec: Some(TaskSpecContainerSpec {
				image: Some(String::from("cloudflare/cloudflared:latest")),
				labels: Some(HashMap::from([(
					String::from("managed-by"),
					String::from("patr"),
				)])),
				command: Some(vec![
					String::from("cloudflared"),
					String::from("tunnel"),
					String::from("run"),
				]),
				env: Some(vec![format!("TUNNEL_TOKEN={}", tunnel_token)]),
				..Default::default()
			}),
			..Default::default()
		}),
		mode: Some(ServiceSpecMode {
			replicated: Some(ServiceSpecModeReplicated { replicas: Some(1) }),
			..Default::default()
		}),
		networks: Some(vec![NetworkAttachmentConfig {
			target: Some(String::from(constants::INGRESS_NETWORK_NAME)),
			..Default::default()
		}]),
		..Default::default()
	})
}
