use std::collections::{BTreeMap, HashMap};

use bollard::{
	auth::DockerCredentials,
	models::{
		HealthConfig,
		Mount,
		MountType,
		NetworkAttachmentConfig,
		Platform,
		ServiceSpec,
		ServiceSpecMode,
		ServiceSpecModeReplicated,
		ServiceSpecUpdateConfig,
		ServiceSpecUpdateConfigFailureActionEnum,
		ServiceSpecUpdateConfigOrderEnum,
		ServiceUpdateStatusStateEnum,
		TaskSpec,
		TaskSpecContainerSpec,
		TaskSpecContainerSpecConfigs,
		TaskSpecContainerSpecFile1,
		TaskSpecPlacement,
		TaskState,
	},
	query_parameters::{ListContainersOptions, ListServicesOptions, ListTasksOptions},
};
use models::api::workspace::{container_registry::*, deployment::*};

use crate::{
	prelude::*,
	runner::{DeploymentService, DockerObjectId, DockerSpec, EnvValue, ServiceTemplate},
};

/// The objects a deployment expects: its volumes and its default-URL routes
/// always, and while it should run, its config mounts and the service that
/// mounts them. A stopped deployment keeps its volumes; its routes answer 503.
pub(crate) fn expected(
	runner: &DockerRunner,
	deployment_id: Uuid,
	local: &LocalState,
) -> ExpectedGraph<DockerObjectId, DockerSpec> {
	let mut graph = ExpectedGraph::new(Owner::Deployment(deployment_id));
	let Some((deployment, running_details)) = local.deployments.get(&deployment_id) else {
		return graph;
	};

	let volume_mounts = running_details
		.volumes
		.keys()
		.map(|path| {
			let volume = DockerObjectId::Volume(utils::deployment_volume_name(deployment_id, path));
			(path.clone(), graph.add(volume, DockerSpec::Volume, []))
		})
		.collect::<BTreeMap<_, _>>();

	if let Some((id, data)) = routes(runner, deployment_id, deployment, running_details) {
		graph.add(id, DockerSpec::Config { data }, []);
	}

	if !deployment.should_run {
		return graph;
	}

	let config_mounts = running_details
		.config_mounts
		.iter()
		.map(|(path, content)| {
			let data = content.as_slice().to_vec();
			let config = DockerObjectId::Config(utils::config_name(
				&format!("patr-config-{deployment_id}"),
				&data,
			));
			(
				path.clone(),
				graph.add(config, DockerSpec::Config { data }, []),
			)
		})
		.collect::<BTreeMap<_, _>>();

	let env = running_details
		.environment_variables
		.iter()
		.map(|(name, value)| {
			let value = match value {
				EnvironmentVariableValue::String(value) => EnvValue::Plain(value.clone()),
				EnvironmentVariableValue::Secret { from_secret } => EnvValue::Secret {
					secret_id: *from_secret,
					last_updated: local.secrets.get(from_secret).copied(),
				},
			};
			(name.clone(), value)
		})
		.collect();

	let depends_on = config_mounts
		.values()
		.chain(volume_mounts.values())
		.cloned()
		.collect::<Vec<_>>();
	graph.add(
		DockerObjectId::Service(format!("patr-{deployment_id}")),
		DockerSpec::Service(Box::new(ServiceTemplate::Deployment(DeploymentService {
			deployment_id,
			registry: deployment.registry.clone(),
			image_tag: deployment.image_tag.clone(),
			current_live_digest: deployment.current_live_digest.clone(),
			env,
			replicas: running_details.min_horizontal_scale,
			startup_probe: running_details.startup_probe.clone(),
			liveness_probe: running_details.liveness_probe.clone(),
			// A deployment with volumes must be `stop-first`: the old task has
			// to release its volumes before the new one mounts them, or both
			// write the same directory at once.
			update_order: if volume_mounts.is_empty() {
				ServiceSpecUpdateConfigOrderEnum::START_FIRST
			} else {
				ServiceSpecUpdateConfigOrderEnum::STOP_FIRST
			},
			config_mounts,
			volume_mounts,
		}))),
		depends_on,
	);

	graph
}

/// The config holding a deployment's default-URL routes, one per HTTP port,
/// or `None` if it exposes no HTTP port (Docker rejects an empty config). A
/// stopped deployment's routes answer 503.
pub(crate) fn routes(
	runner: &DockerRunner,
	deployment_id: Uuid,
	deployment: &Deployment,
	running_details: &DeploymentRunningDetails,
) -> Option<(DockerObjectId, Vec<u8>)> {
	let data = running_details
		.ports
		.iter()
		.filter(|(_, port_type)| matches!(port_type, ExposedPortType::Http))
		.map(|(port, _)| {
			ingress::generate_config_for_deployment(
				deployment_id,
				port.value(),
				deployment.should_run,
				runner.settings.data.runner_exposure_type.is_private(),
			)
		})
		.collect::<String>();
	if data.is_empty() {
		return None;
	}
	Some((
		DockerObjectId::Config(utils::config_name(
			&format!("patr-ingress-{deployment_id}"),
			&data,
		)),
		data.into_bytes(),
	))
}

/// Creates or updates a deployment's service, resolving the image, the secret
/// values and the IDs of its config mounts.
pub(crate) async fn apply(
	runner: &DockerRunner,
	owner: Owner,
	name: &str,
	service: &DeploymentService,
	hash: &str,
) -> Result<(), RunnerError> {
	let DockerRunner {
		docker, settings, ..
	} = runner;
	let RunnerSettings {
		workspace_id,
		runner_id,
		api_token,
		user_agent,
		..
	} = settings;
	let deployment_id = service.deployment_id;

	// Patr-registry deployments only carry a `repository_id` locally, so the
	// repository's name comes from the API.
	let image_path = match &service.registry {
		DeploymentRegistry::PatrRegistry { repository_id, .. } => {
			let repository = client::make_request(
				ApiRequest::<GetContainerRepositoryInfoRequest>::builder()
					.path(GetContainerRepositoryInfoPath {
						workspace_id: *workspace_id,
						repository_id: *repository_id,
					})
					.headers(GetContainerRepositoryInfoRequestHeaders {
						authorization: api_token.clone(),
						user_agent: user_agent.clone(),
					})
					.query(())
					.body(GetContainerRepositoryInfoRequest)
					.build(),
			)
			.await
			.map_err(|err| err.body.error)?
			.body
			.repository;
			format!(
				"{}/{}/{}",
				service.registry.registry_url(),
				workspace_id,
				repository.name
			)
		}
		DeploymentRegistry::ExternalRegistry {
			registry,
			image_name,
		} => format!("{registry}/{image_name}"),
	};
	let image = match &service.current_live_digest {
		Some(digest) => format!("{image_path}@{digest}"),
		None => format!("{image_path}:{}", service.image_tag),
	};

	// Patr's credentials only ever go to Patr's registry. External images are
	// pulled anonymously.
	let registry_auth = match &service.registry {
		DeploymentRegistry::PatrRegistry { .. } => Some(DockerCredentials {
			username: Some("patr".to_string()),
			password: Some(api_token.0.token().to_string()),
			serveraddress: Some(service.registry.registry_url()),
			..Default::default()
		}),
		DeploymentRegistry::ExternalRegistry { .. } => None,
	};

	// The platforms the image supports, so Swarm only schedules it on a node
	// that can run it. A lookup that fails or finds nothing sets no
	// constraint, and Swarm reports what happens.
	let platforms = match &service.registry {
		DeploymentRegistry::PatrRegistry { repository_id, .. } => client::make_request(
			ApiRequest::<GetContainerRepositoryManifestDetailsRequest>::builder()
				.path(GetContainerRepositoryManifestDetailsPath {
					workspace_id: *workspace_id,
					repository_id: *repository_id,
					digest_or_tag: service
						.current_live_digest
						.clone()
						.unwrap_or_else(|| service.image_tag.clone()),
				})
				.headers(GetContainerRepositoryManifestDetailsRequestHeaders {
					authorization: api_token.clone(),
					user_agent: user_agent.clone(),
				})
				.query(())
				.body(GetContainerRepositoryManifestDetailsRequest)
				.build(),
		)
		.await
		.map(|response| {
			response
				.body
				.manifest_details
				.platforms
				.into_iter()
				.filter(|platform| platform.os != "unknown")
				.map(|platform| Platform {
					architecture: Some(platform.architecture),
					os: Some(platform.os),
				})
				.collect::<Vec<_>>()
		})
		.unwrap_or_default(),
		DeploymentRegistry::ExternalRegistry { .. } => {
			let cached = runner
				.platforms
				.lock()
				.ok()
				.and_then(|platforms| platforms.get(&image).cloned());
			match cached {
				Some(platforms) => platforms,
				None => {
					let platforms = docker
						.inspect_registry_image(&image, None)
						.await
						.map(|inspect| {
							inspect
								.platforms
								.into_iter()
								.map(|platform| Platform {
									architecture: platform.architecture,
									os: platform.os,
								})
								.collect::<Vec<_>>()
						})
						.unwrap_or_default();
					if service.current_live_digest.is_some() &&
						let Ok(mut cache) = runner.platforms.lock()
					{
						cache.insert(image.clone(), platforms.clone());
					}
					platforms
				}
			}
		}
	};

	// Swarm supports a single healthcheck, so the liveness probe wins over the
	// startup probe.
	let health_check = service
		.liveness_probe
		.as_ref()
		.or(service.startup_probe.as_ref())
		.map(|probe| HealthConfig {
			test: Some(vec![
				String::from("CMD-SHELL"),
				format!(
					"curl -sf http://localhost:{}{} || exit 1",
					probe.port, probe.path
				),
			]),
			interval: Some(30_000_000_000),
			timeout: Some(10_000_000_000),
			retries: Some(3),
			start_period: if service.startup_probe.is_some() {
				Some(60_000_000_000)
			} else {
				Some(10_000_000_000)
			},
			..Default::default()
		});

	let host_ip = docker
		.inspect_network("bridge", None)
		.await
		.ok()
		.and_then(|network| network.ipam)
		.and_then(|ipam| ipam.config)
		.and_then(|configs| configs.into_iter().next())
		.and_then(|config| config.gateway)
		.unwrap_or_else(|| String::from("172.17.0.1"));

	let mut configs = Vec::with_capacity(service.config_mounts.len());
	for (path, config) in &service.config_mounts {
		let DockerObjectId::Config(config_name) = config else {
			continue;
		};
		let config_id = docker
			.inspect_config(config_name)
			.await
			.map_err(RunnerError::host)?
			.id;
		configs.push(TaskSpecContainerSpecConfigs {
			file: Some(TaskSpecContainerSpecFile1 {
				name: Some(path.clone()),
				mode: Some(0o444),
				uid: Some(String::from("0")),
				gid: Some(String::from("0")),
			}),
			config_id,
			config_name: Some(config_name.clone()),
			runtime: None,
		});
	}

	let mounts = service
		.volume_mounts
		.iter()
		.filter_map(|(path, volume)| {
			let DockerObjectId::Volume(volume_name) = volume else {
				return None;
			};
			Some(Mount {
				target: Some(path.clone()),
				source: Some(volume_name.clone()),
				typ: Some(MountType::VOLUME),
				read_only: Some(false),
				..Default::default()
			})
		})
		.collect::<Vec<_>>();

	let mut env = Vec::with_capacity(service.env.len());
	for (key, value) in &service.env {
		let value = match value {
			EnvValue::Plain(value) => value.clone(),
			EnvValue::Secret { secret_id, .. } => {
				secrets::get_secret_value(*runner_id, api_token, *workspace_id, *secret_id).await?
			}
		};
		env.push(format!("{key}={value}"));
	}

	let mut labels = utils::object_labels(owner, hash);
	labels.insert(String::from("patr.deploymentId"), deployment_id.to_string());

	let spec = ServiceSpec {
		name: Some(String::from(name)),
		labels: Some(labels),
		task_template: Some(TaskSpec {
			container_spec: Some(TaskSpecContainerSpec {
				image: Some(image),
				hostname: Some(format!("{deployment_id}.onpatr.cloud")),
				env: Some(env),
				// Containers only carry what Alloy, cAdvisor and `observe`
				// read: anything else here would restart every task when it
				// changes.
				labels: Some(HashMap::from([
					(String::from("managed-by"), String::from("patr")),
					(String::from("patr.deploymentId"), deployment_id.to_string()),
				])),
				health_check,
				hosts: Some(vec![format!("{host_ip} host.docker.internal")]),
				mounts: (!mounts.is_empty()).then_some(mounts),
				configs: (!configs.is_empty()).then_some(configs),
				..Default::default()
			}),
			networks: Some(vec![NetworkAttachmentConfig {
				target: Some(String::from(constants::INGRESS_NETWORK_NAME)),
				aliases: Some(vec![format!("{deployment_id}.onpatr.local")]),
				driver_opts: None,
			}]),
			placement: (!platforms.is_empty()).then(|| TaskSpecPlacement {
				platforms: Some(platforms),
				..Default::default()
			}),
			..Default::default()
		}),
		mode: Some(ServiceSpecMode {
			replicated: Some(ServiceSpecModeReplicated {
				replicas: Some(service.replicas.into()),
			}),
			..Default::default()
		}),
		update_config: Some(ServiceSpecUpdateConfig {
			order: Some(service.update_order),
			failure_action: Some(ServiceSpecUpdateConfigFailureActionEnum::PAUSE),
			..Default::default()
		}),
		..Default::default()
	};

	utils::upsert_service(docker, spec, registry_auth).await
}

/// A deployment's observed status, from its service, or `Stopped` if it has
/// none. The first rule that matches wins: an image that can't run on any
/// node, an image that couldn't be pulled, an unhealthy container, a crash
/// loop, an update in progress, and otherwise running.
pub(crate) async fn observe(
	DockerRunner { docker, .. }: &DockerRunner,
	deployment_id: Uuid,
) -> Result<ObservedDeploymentStatus, RunnerError> {
	let Some(service) = docker
		.list_services(Some(ListServicesOptions {
			filters: Some(HashMap::from([(
				String::from("label"),
				vec![
					String::from("managed-by=patr"),
					format!("patr.deploymentId={deployment_id}"),
				],
			)])),
			status: Some(true),
		}))
		.await
		.map_err(RunnerError::host)?
		.into_iter()
		.next()
	else {
		return Ok(ObservedDeploymentStatus::Stopped);
	};

	let tasks = docker
		.list_tasks(Some(ListTasksOptions {
			filters: Some(HashMap::from([(
				String::from("service"),
				vec![format!("patr-{deployment_id}")],
			)])),
		}))
		.await
		.map_err(RunnerError::host)?;

	let unhealthy = !docker
		.list_containers(Some(ListContainersOptions {
			filters: Some(HashMap::from([
				(String::from("health"), vec![String::from("unhealthy")]),
				(
					String::from("label"),
					vec![format!("patr.deploymentId={deployment_id}")],
				),
			])),
			..Default::default()
		}))
		.await
		.map_err(RunnerError::host)?
		.is_empty();

	// Tasks created since the service was last updated run its current
	// spec. Older ones are history from earlier versions.
	let current_tasks = tasks
		.iter()
		.filter(|task| task.service_id.is_some() && task.service_id == service.id)
		.filter(|task| {
			task.created_at
				.zip(service.updated_at)
				.is_none_or(|(created, updated)| created >= updated)
		})
		.filter_map(|task| task.status.as_ref())
		.collect::<Vec<_>>();
	let has_task = |state: TaskState, text: &str| {
		current_tasks.iter().any(|status| {
			status.state == Some(state) &&
				[&status.err, &status.message]
					.into_iter()
					.flatten()
					.any(|message| message.contains(text))
		})
	};

	let running = service
		.service_status
		.as_ref()
		.and_then(|status| status.running_tasks)
		.unwrap_or(0);
	let desired = service
		.service_status
		.as_ref()
		.and_then(|status| status.desired_tasks)
		.unwrap_or(0);
	let updating = service
		.update_status
		.as_ref()
		.and_then(|status| status.state)
		.is_some_and(|state| state == ServiceUpdateStatusStateEnum::UPDATING);

	let status = if has_task(TaskState::PENDING, "unsupported platform") {
		ObservedDeploymentStatus::InvalidArchitecture
	} else if has_task(TaskState::REJECTED, "No such image") {
		ObservedDeploymentStatus::ImagePullFailed
	} else if unhealthy || has_task(TaskState::FAILED, "unhealthy") {
		ObservedDeploymentStatus::HealthCheckFailing
	} else if current_tasks
		.iter()
		.filter(|status| matches!(status.state, Some(TaskState::FAILED | TaskState::REJECTED)))
		.count() >=
		2
	{
		ObservedDeploymentStatus::Crashing
	} else if running < desired || updating {
		ObservedDeploymentStatus::Deploying
	} else {
		ObservedDeploymentStatus::Running
	};

	Ok(status)
}
