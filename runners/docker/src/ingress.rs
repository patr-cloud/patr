use std::{collections::HashMap, io::Error as IoError, iter};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use bollard::{
	Docker,
	models::{ConfigSpec, Mount, MountType},
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
	// Always deploy Caddy as the ingress service, regardless of exposure type
	let ingress_service_spec = build_ingress_spec(docker, settings).await?;

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

	// For private runners, also deploy the Cloudflare tunnel as a separate service
	if settings.data.runner_exposure_type.is_private() {
		let tunnel_service_spec = get_cloudflare_spec(docker, settings).await?;

		let tunnel = docker
			.inspect_service(constants::TUNNEL_SERVICE_NAME, None)
			.await
			.ok();

		if let Some(version) = tunnel
			.and_then(|tunnel| tunnel.version)
			.and_then(|version| version.index)
		{
			docker
				.update_service(
					constants::TUNNEL_SERVICE_NAME,
					tunnel_service_spec,
					UpdateServiceOptionsBuilder::new()
						.version(version as i32)
						.build(),
					None,
				)
				.await
				.map_err(RunnerError::host)?;
		} else {
			docker
				.create_service(tunnel_service_spec, None)
				.await
				.map_err(RunnerError::host)?;
		}
	}

	Ok(())
}

/// Generate the Caddyfile config for a deployment, given the deployment ID and
/// the port the deployment is listening on. This will read the Caddyfile
/// template from the assets folder and replace the placeholders with the actual
/// values.
pub fn generate_config_for_deployment(deployment_id: Uuid, port: u16, is_private: bool) -> String {
	format!(
		include_str!("../../../assets/runner/Caddyfile.deployment-default-url.template"),
		scheme = if is_private { "http://" } else { "" },
		deployment_id = deployment_id,
		port = port
	)
}

/// Update the cloudflare tunnel token for the ingress service, if the tunnel
/// token config has changed. This is done by first checking if the config
/// exists, and if it does, deleting it and creating a new one with the updated
/// token. Then, the ingress service is updated to pick up the new config.
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
					String::from("patr.version"),
					String::from(constants::PATR_VERSION),
				),
				(
					String::from("patr.deploymentId"),
					String::from(constants::INGRESS_SERVICE_NAME),
				),
			])),
			data: Some(BASE64.encode(new_token.as_bytes())),
			templating: None,
		})
		.await
		.map_err(RunnerError::host)?;

	Ok(())
}

/// Delete a deployment's ingress config. This rebuilds Caddy without the
/// deployment's config (unmounting it), then deletes the Docker config.
pub async fn delete_deployment_config(
	docker: &Docker,
	settings: &RunnerSettings<DockerSettings>,
	deployment_id: Uuid,
) -> Result<(), RunnerError> {
	// Find the deployment's configs by label
	let deployment_configs = docker
		.list_configs(Some(ListConfigsOptions {
			filters: Some(HashMap::from([(
				String::from("label"),
				vec![format!("patr.deploymentId={}", deployment_id)],
			)])),
		}))
		.await
		.map_err(RunnerError::host)?;

	let config_ids_to_remove = deployment_configs
		.iter()
		.filter_map(|c| c.id.clone())
		.collect::<Vec<_>>();

	// Build the ingress spec and remove this deployment's configs from
	// the mount list so that updating the service unmounts them.
	let mut ingress_service_spec = build_ingress_spec(docker, settings).await?;

	if let Some(configs) = ingress_service_spec
		.task_template
		.as_mut()
		.and_then(|task| task.container_spec.as_mut())
		.and_then(|container| container.configs.as_mut())
	{
		configs.retain(|c| {
			c.config_id
				.as_ref()
				.is_none_or(|id| !config_ids_to_remove.contains(id))
		});
	}

	// Update Caddy — this unmounts the configs
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
	}

	// Now the configs are unmounted — safe to delete
	for config in deployment_configs {
		if let Some(id) = config.id {
			docker.delete_config(&id).await.map_err(|err| {
				error!("Error removing config {}: {}", id, err);
				RunnerError::host(err)
			})?;
		}
	}

	Ok(())
}

/// Build the service spec for the ingress service, with the latest
/// deployment configs mounted in the service.
pub(crate) async fn build_ingress_spec(
	docker: &Docker,
	settings: &RunnerSettings<DockerSettings>,
) -> Result<ServiceSpec, RunnerError> {
	// Per-deployment Caddyfile snippets are uniquely identified by
	// `patr.configBaseName` starting with `ingress-`; per-managed-URL by
	// `managed-url-`. Data config-mounts use `config-{id}-{N}`, and the main
	// Caddyfile.base uses `patr-ingress-config` (mounted separately at
	// /etc/caddy/Caddyfile, not under /etc/caddy/urls).
	let config_ids = docker
		.list_configs(Some(ListConfigsOptions {
			filters: Some(HashMap::from([(
				String::from("label"),
				vec![String::from("patr.configBaseName")],
			)])),
		}))
		.await
		.map_err(|err| {
			error!("Error listing configs: {:?}", err);
			RunnerError::host(err)
		})?
		.into_iter()
		.filter_map(|config| {
			let labels = config.spec.as_ref()?.labels.as_ref()?;
			let base_name = labels.get("patr.configBaseName")?;
			let resource_id = if base_name.starts_with("ingress-") {
				labels.get("patr.deploymentId")?.parse::<Uuid>().ok()?
			} else if base_name.starts_with("managed-url-") {
				labels.get("patr.managedUrlId")?.parse::<Uuid>().ok()?
			} else {
				return None;
			};
			Some((
				resource_id,
				config.id.clone()?,
				config.spec.as_ref()?.name.clone()?,
			))
		})
		.collect::<Vec<_>>();

	let base_ingress_config = include_str!("../../../assets/runner/Caddyfile.base");
	let base_config = Base64String::from_string(base_ingress_config.to_string());

	let (ingress_config_id, ingress_config_name) = crate::utils::update_config(
		docker,
		constants::INGRESS_CONFIG_NAME,
		HashMap::from([
			(String::from("managed-by"), String::from("patr")),
			(
				String::from("patr.deploymentId"),
				String::from(constants::INGRESS_SERVICE_NAME),
			),
		]),
		base_config.to_string(),
	)
	.await?;

	let networks = Some(vec![NetworkAttachmentConfig {
		target: Some(String::from(constants::INGRESS_NETWORK_NAME)),
		aliases: Some(vec![String::from("patr-ingress"), String::from("ingress")]),
		driver_opts: None,
	}]);

	Ok(ServiceSpec {
		name: Some(String::from(constants::INGRESS_SERVICE_NAME)),
		labels: Some(HashMap::from([
			(String::from("managed-by"), String::from("patr")),
			(
				String::from("patr.version"),
				String::from(constants::PATR_VERSION),
			),
		])),
		task_template: Some(TaskSpec {
			plugin_spec: None,
			container_spec: Some(TaskSpecContainerSpec {
				image: Some(String::from("caddy:2")),
				labels: Some(HashMap::from([
					(String::from("managed-by"), String::from("patr")),
					(
						String::from("patr.version"),
						String::from(constants::PATR_VERSION),
					),
				])),
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
						.map(
							|(resource_id, config_id, config_name)| TaskSpecContainerSpecConfigs {
								file: Some(TaskSpecContainerSpecFile1 {
									name: Some(format!("/etc/caddy/urls/{}.caddy", resource_id)),
									mode: Some(0o444),
									uid: Some("0".to_string()),
									gid: Some("0".to_string()),
								}),
								config_id: Some(config_id),
								config_name: Some(config_name),
								runtime: None,
							},
						)
						.chain(iter::once(TaskSpecContainerSpecConfigs {
							file: Some(TaskSpecContainerSpecFile1 {
								name: Some(String::from("/etc/caddy/Caddyfile")),
								mode: Some(0o444),
								uid: Some("0".to_string()),
								gid: Some("0".to_string()),
							}),
							config_id: Some(ingress_config_id),
							config_name: Some(ingress_config_name),
							runtime: None,
						}))
						.collect(),
				),
				// Mount a named volume to store the TLS certs, so they persist across service
				// updates and restarts
				mounts: Some(vec![Mount {
					target: Some(String::from("/data")),
					source: Some(String::from(constants::INGRESS_TLS_CERTS_VOLUME_NAME)),
					typ: Some(MountType::VOLUME),
					read_only: Some(false),
					..Default::default()
				}]),
				..Default::default()
			}),
			networks,
			..Default::default()
		}),
		mode: Some(ServiceSpecMode {
			replicated: Some(ServiceSpecModeReplicated { replicas: Some(1) }),
			..Default::default()
		}),
		// Only publish ports for public runners — private runners receive
		// traffic through the Cloudflare tunnel, not directly
		endpoint_spec: if settings.data.runner_exposure_type.is_private() {
			None
		} else {
			Some(EndpointSpec {
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
			})
		},
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

	let networks = Some(vec![NetworkAttachmentConfig {
		target: Some(String::from(constants::INGRESS_NETWORK_NAME)),
		..Default::default()
	}]);

	Ok(ServiceSpec {
		name: Some(String::from(constants::TUNNEL_SERVICE_NAME)),
		labels: Some(HashMap::from([
			(String::from("managed-by"), String::from("patr")),
			(
				String::from("patr.version"),
				String::from(constants::PATR_VERSION),
			),
		])),
		task_template: Some(TaskSpec {
			plugin_spec: None,
			container_spec: Some(TaskSpecContainerSpec {
				image: Some(String::from("cloudflare/cloudflared:latest")),
				labels: Some(HashMap::from([
					(String::from("managed-by"), String::from("patr")),
					(
						String::from("patr.version"),
						String::from(constants::PATR_VERSION),
					),
				])),
				command: Some(vec![
					String::from("cloudflared"),
					String::from("tunnel"),
					String::from("run"),
				]),
				env: Some(vec![format!("TUNNEL_TOKEN={}", tunnel_token)]),
				..Default::default()
			}),
			networks,
			..Default::default()
		}),
		mode: Some(ServiceSpecMode {
			replicated: Some(ServiceSpecModeReplicated { replicas: Some(1) }),
			..Default::default()
		}),
		..Default::default()
	})
}
