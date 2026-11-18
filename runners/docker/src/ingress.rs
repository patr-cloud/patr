use std::collections::{BTreeMap, HashMap};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use bollard::{
	Docker,
	errors::Error as DockerError,
	models::{ConfigSpec, Mount, MountType},
	service::{
		EndpointPortConfig,
		EndpointPortConfigProtocolEnum,
		EndpointPortConfigPublishModeEnum,
		EndpointSpec,
		NetworkAttachmentConfig,
		ServiceSpec,
		ServiceSpecMode,
		ServiceSpecModeReplicated,
		ServiceSpecUpdateConfig,
		ServiceSpecUpdateConfigOrderEnum,
		TaskSpec,
		TaskSpecContainerSpec,
		TaskSpecContainerSpecConfigs,
		TaskSpecContainerSpecFile1,
	},
};

use crate::{
	prelude::*,
	runner::{DockerObjectId, DockerSpec, IngressService, ServiceTemplate},
};

/// The objects the runner itself expects: the overlay network, the Caddy
/// ingress with its configs, error pages and certificate volume, the tunnel
/// for private runners once it has a token, and Alloy.
///
/// Caddy mounts the routes of every deployment and managed URL in the local
/// state. Those configs belong to their own owners, so they aren't nodes here:
/// Caddy mounts the ones that exist when it's applied.
pub(crate) fn expected(
	runner: &DockerRunner,
	local: &LocalState,
	tunnel_token_hash: Option<&str>,
) -> ExpectedGraph<DockerObjectId, DockerSpec> {
	let settings = &runner.settings.data;
	let mut graph = ExpectedGraph::new(Owner::System);

	let network = graph.add(
		DockerObjectId::Network(String::from(constants::INGRESS_NETWORK_NAME)),
		DockerSpec::Network {
			ipv6: settings.enable_ipv6,
		},
		[],
	);

	let base = include_bytes!("../../../assets/runner/Caddyfile.base").to_vec();
	let base_config = graph.add(
		DockerObjectId::Config(utils::config_name(constants::INGRESS_CONFIG_NAME, &base)),
		DockerSpec::Config { data: base },
		[],
	);

	let error_pages = [
		(
			"/etc/caddy/pages/503.html",
			"patr-error-page-503",
			include_bytes!("../../../assets/error-pages/deployment-stopped.html").as_slice(),
		),
		(
			"/etc/caddy/pages/502.html",
			"patr-error-page-502",
			include_bytes!("../../../assets/error-pages/connection-error.html").as_slice(),
		),
	]
	.into_iter()
	.map(|(path, base, data)| {
		let page = graph.add(
			DockerObjectId::Config(utils::config_name(base, data)),
			DockerSpec::Config {
				data: data.to_vec(),
			},
			[],
		);
		(String::from(path), page)
	})
	.collect::<BTreeMap<_, _>>();

	let data_volume = graph.add(
		DockerObjectId::Volume(String::from(constants::INGRESS_TLS_CERTS_VOLUME_NAME)),
		DockerSpec::Volume,
		[],
	);

	let imports = local
		.deployments
		.iter()
		.filter_map(|(deployment_id, (deployment, running_details))| {
			deployment::routes(runner, *deployment_id, deployment, running_details)
				.map(|(id, _)| id)
		})
		.chain(
			local
				.managed_urls
				.iter()
				.filter_map(|(managed_url_id, managed_url)| {
					let (deployment, _) = local.deployments.get(&managed_url.deployment_id)?;
					managed_url::routes(runner, *managed_url_id, managed_url, deployment.should_run)
						.ok()
						.map(|(id, _)| id)
				}),
		)
		.collect();

	let depends_on = [network.clone(), base_config.clone(), data_volume.clone()]
		.into_iter()
		.chain(error_pages.values().cloned())
		.collect::<Vec<_>>();
	graph.add(
		DockerObjectId::Service(String::from(constants::INGRESS_SERVICE_NAME)),
		DockerSpec::Service(Box::new(ServiceTemplate::Ingress(IngressService {
			base_config,
			error_pages,
			imports,
			data_volume,
			published_ports: (!settings.runner_exposure_type.is_private()).then_some((
				settings.ingress_http_listen_port,
				settings.ingress_https_listen_port,
			)),
		}))),
		depends_on,
	);

	if settings.runner_exposure_type.is_private() &&
		let Some(token_hash) = tunnel_token_hash
	{
		graph.add(
			DockerObjectId::Service(String::from(constants::TUNNEL_SERVICE_NAME)),
			DockerSpec::Service(Box::new(ServiceTemplate::Tunnel {
				token_hash: String::from(token_hash),
			})),
			[network.clone()],
		);
	}

	let alloy = alloy::generate_alloy_config(&runner.settings).into_bytes();
	let alloy_config = graph.add(
		DockerObjectId::Config(utils::config_name(constants::ALLOY_CONFIG_NAME, &alloy)),
		DockerSpec::Config { data: alloy },
		[],
	);
	graph.add(
		DockerObjectId::Service(String::from(constants::ALLOY_SERVICE_NAME)),
		DockerSpec::Service(Box::new(ServiceTemplate::Alloy {
			config: alloy_config.clone(),
		})),
		[network, alloy_config],
	);

	graph
}

/// Generate the Caddyfile route for a deployment's default URL on one port. A
/// stopped deployment's route answers 503, which Caddy serves as the
/// "deployment stopped" page.
pub fn generate_config_for_deployment(
	deployment_id: Uuid,
	port: u16,
	should_run: bool,
	is_private: bool,
) -> String {
	format!(
		include_str!("../../../assets/runner/Caddyfile.deployment-default-url.template"),
		scheme = if is_private { "http://" } else { "" },
		deployment_id = deployment_id,
		port = port,
		handler = if should_run {
			format!("reverse_proxy {deployment_id}.onpatr.local:{port}")
		} else {
			String::from("error 503")
		},
	)
}

/// Read the tunnel token from its config, if it has one.
pub(crate) async fn tunnel_token(docker: &Docker) -> Result<Option<String>, RunnerError> {
	// `inspect_config` returns the data base64-encoded.
	let data = match docker
		.inspect_config(constants::TUNNEL_TOKEN_CONFIG_NAME)
		.await
	{
		Ok(config) => config.spec.and_then(|spec| spec.data),
		Err(DockerError::DockerResponseServerError {
			status_code: 404, ..
		}) => None,
		Err(err) => return Err(RunnerError::host(err)),
	};
	data.map(|data| {
		String::from_utf8(BASE64.decode(data).map_err(RunnerError::host)?)
			.map_err(RunnerError::host)
	})
	.transpose()
}

/// Store the tunnel token in its config, replacing the old one. Returns
/// whether it changed. The tunnel reads the token when it's applied rather
/// than mounting the config, so the config can be replaced while it runs.
pub(crate) async fn update_tunnel_token(
	docker: &Docker,
	token: String,
) -> Result<bool, RunnerError> {
	if tunnel_token(docker).await?.as_deref() == Some(token.as_str()) {
		return Ok(false);
	}

	match docker
		.delete_config(constants::TUNNEL_TOKEN_CONFIG_NAME)
		.await
	{
		Ok(()) |
		Err(DockerError::DockerResponseServerError {
			status_code: 404, ..
		}) => {}
		Err(err) => return Err(RunnerError::host(err)),
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
			])),
			data: Some(BASE64.encode(token.as_bytes())),
			templating: None,
		})
		.await
		.map_err(RunnerError::host)?;

	Ok(true)
}

/// Creates or updates the Caddy ingress. Routes that don't exist yet are left
/// out, and the service is stamped with a hash that doesn't match, so the next
/// reconcile tries again.
pub(crate) async fn apply_ingress(
	runner: &DockerRunner,
	owner: Owner,
	name: &str,
	service: &IngressService,
	hash: &str,
) -> Result<(), RunnerError> {
	let docker = &runner.docker;

	let mut configs = Vec::new();
	let mut missing = false;
	// Routes are mounted under their config name, which is unique. They belong
	// to other owners, so the ones that don't exist yet are skipped.
	let mounts = [(&service.base_config, Some("/etc/caddy/Caddyfile"))]
		.into_iter()
		.chain(
			service
				.error_pages
				.iter()
				.map(|(path, page)| (page, Some(path.as_str()))),
		)
		.chain(service.imports.iter().map(|route| (route, None)));
	for (config, path) in mounts {
		let DockerObjectId::Config(config_name) = config else {
			continue;
		};
		let config_id = match docker.inspect_config(config_name).await {
			Ok(config) => config.id,
			Err(DockerError::DockerResponseServerError {
				status_code: 404, ..
			}) if path.is_none() => {
				missing = true;
				continue;
			}
			Err(err) => return Err(RunnerError::host(err)),
		};
		configs.push(TaskSpecContainerSpecConfigs {
			file: Some(TaskSpecContainerSpecFile1 {
				name: Some(path.map_or_else(
					|| format!("/etc/caddy/urls/{config_name}.caddy"),
					String::from,
				)),
				mode: Some(0o444),
				uid: Some(String::from("0")),
				gid: Some(String::from("0")),
			}),
			config_id,
			config_name: Some(config_name.clone()),
			runtime: None,
		});
	}

	let hash = if missing {
		format!("{hash}-partial")
	} else {
		String::from(hash)
	};

	let DockerObjectId::Volume(data_volume) = &service.data_volume else {
		return Err(RunnerError::host(
			"the ingress's data volume isn't a volume",
		));
	};

	let spec = ServiceSpec {
		name: Some(String::from(name)),
		labels: Some(utils::object_labels(owner, &hash)),
		task_template: Some(TaskSpec {
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
				configs: Some(configs),
				// The certificates outlive service updates and restarts.
				mounts: Some(vec![Mount {
					target: Some(String::from("/data")),
					source: Some(data_volume.clone()),
					typ: Some(MountType::VOLUME),
					read_only: Some(false),
					..Default::default()
				}]),
				..Default::default()
			}),
			networks: Some(vec![NetworkAttachmentConfig {
				target: Some(String::from(constants::INGRESS_NETWORK_NAME)),
				aliases: Some(vec![String::from("patr-ingress"), String::from("ingress")]),
				driver_opts: None,
			}]),
			..Default::default()
		}),
		mode: Some(ServiceSpecMode {
			replicated: Some(ServiceSpecModeReplicated { replicas: Some(1) }),
			..Default::default()
		}),
		// Start the new Caddy before stopping the old one, so a route change
		// never drops traffic.
		update_config: Some(ServiceSpecUpdateConfig {
			order: Some(ServiceSpecUpdateConfigOrderEnum::START_FIRST),
			..Default::default()
		}),
		// Private runners receive traffic through the tunnel, so only public
		// runners publish ports.
		endpoint_spec: service.published_ports.map(|(http, https)| EndpointSpec {
			mode: None,
			ports: Some(vec![
				EndpointPortConfig {
					name: None,
					protocol: Some(EndpointPortConfigProtocolEnum::TCP),
					target_port: Some(80),
					published_port: Some(http.into()),
					publish_mode: Some(EndpointPortConfigPublishModeEnum::INGRESS),
				},
				EndpointPortConfig {
					name: None,
					protocol: Some(EndpointPortConfigProtocolEnum::TCP),
					target_port: Some(443),
					published_port: Some(https.into()),
					publish_mode: Some(EndpointPortConfigPublishModeEnum::INGRESS),
				},
			]),
		}),
		..Default::default()
	};

	utils::upsert_service(docker, spec, None).await
}

/// Creates or updates the Cloudflare tunnel, with the token from its config.
pub(crate) async fn apply_tunnel(
	runner: &DockerRunner,
	owner: Owner,
	name: &str,
	hash: &str,
) -> Result<(), RunnerError> {
	let token = tunnel_token(&runner.docker)
		.await?
		.ok_or_else(|| RunnerError::host("the runner has no tunnel token yet"))?;

	let spec = ServiceSpec {
		name: Some(String::from(name)),
		labels: Some(utils::object_labels(owner, hash)),
		task_template: Some(TaskSpec {
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
				env: Some(vec![format!("TUNNEL_TOKEN={token}")]),
				..Default::default()
			}),
			networks: Some(vec![NetworkAttachmentConfig {
				target: Some(String::from(constants::INGRESS_NETWORK_NAME)),
				..Default::default()
			}]),
			..Default::default()
		}),
		mode: Some(ServiceSpecMode {
			replicated: Some(ServiceSpecModeReplicated { replicas: Some(1) }),
			..Default::default()
		}),
		..Default::default()
	};

	utils::upsert_service(&runner.docker, spec, None).await
}
