use std::collections::HashMap;

use bollard::{
	auth::DockerCredentials,
	models::{
		HealthConfig,
		NetworkAttachmentConfig,
		ServiceSpec,
		ServiceSpecMode,
		ServiceSpecModeReplicated,
		TaskSpec,
		TaskSpecContainerSpec,
		TaskSpecContainerSpecConfigs,
		TaskSpecContainerSpecFile1,
	},
	query_parameters::{
		ListConfigsOptions,
		ListServicesOptions,
		ListServicesOptionsBuilder,
		UpdateServiceOptionsBuilder,
	},
};
use futures::{Stream, StreamExt};
use models::api::workspace::deployment::*;

use crate::prelude::*;

/// Upsert (create or update) a deployment. This will create the service if
/// it does not exist, or update the service if it does exist.
pub(crate) async fn upsert(
	DockerRunner { docker, settings }: &DockerRunner,
	WithId {
		id,
		data:
			Deployment {
				name,
				registry,
				image_tag,
				status: _,       // TODO handle paused deployments
				machine_type: _, // TODO
				runner: _,
				current_live_digest,
			},
	}: WithId<Deployment>,
	DeploymentRunningDetails {
		deploy_on_push: _,
		min_horizontal_scale,
		max_horizontal_scale,
		ports,
		environment_variables,
		startup_probe,
		liveness_probe,
		config_mounts,
		volumes: _, // TODO
	}: DeploymentRunningDetails,
) -> Result<(), RunnerError> {
	let service_name = format!("patr-{}", id);

	// Check if the service exists
	let existing_service = docker.inspect_service(&service_name, None).await.ok();

	let image = format!(
		"{}/{}{}{}",
		registry.registry_url(),
		registry.image_name().unwrap(),
		if current_live_digest.is_some() {
			'@'
		} else {
			':'
		},
		current_live_digest.as_deref().unwrap_or(&image_tag)
	);

	// Build registry credentials for both image pull and Swarm task scheduling.
	// The local DB stores all registries as ExternalRegistry, so check the URL
	// string rather than the enum variant to detect Patr registry images.
	let is_patr = registry.registry_url() == models::utils::constants::CONTAINER_REGISTRY_URL;
	let registry_auth = if let RunnerMode::Managed {
		workspace_id: _,
		runner_id: _,
		user_agent: _,
		api_token,
	} = &settings.mode &&
		is_patr
	{
		Some(DockerCredentials {
			username: Some("patr".to_string()),
			password: Some(api_token.0.token().to_string()),
			serveraddress: Some(registry.registry_url()),
			..Default::default()
		})
	} else {
		None
	};

	// Build health check from probes. Docker Swarm only supports a single
	// healthcheck, so we prefer liveness_probe over startup_probe if both are
	// provided. The health check will be used for container health monitoring.
	let health_check = liveness_probe
		.as_ref()
		.or(startup_probe.as_ref())
		.map(|probe| HealthConfig {
			test: Some(vec![
				String::from("CMD-SHELL"),
				format!(
					"curl -sf http://localhost:{}{} || exit 1",
					probe.port, probe.path
				),
			]),
			// Default interval of 30 seconds
			interval: Some(30_000_000_000), // 30s in nanoseconds
			// Default timeout of 10 seconds
			timeout: Some(10_000_000_000), // 10s in nanoseconds
			// 3 retries before marking unhealthy
			retries: Some(3),
			// Start period - give container time to start before health checks begin
			// Use startup_probe interval if available, otherwise default to 60s
			start_period: if startup_probe.is_some() {
				Some(60_000_000_000) // 60s in nanoseconds
			} else {
				Some(10_000_000_000) // 10s in nanoseconds
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

	// Materialize config mounts as Docker Swarm configs and attach them to the
	// container spec. Each mount gets its own config with base_name
	// `config-{id}-{N}` where N is the ordinal after sorting the mounts by path
	// — explicit sort keeps the ordinal stable regardless of map type.
	// Per-ordinal base_name means update_config's cleanup is scoped to this
	// mount only (so siblings survive) and same-content-at-different-paths
	// never clashes on the final Docker config name.
	let mount_count = config_mounts.len();
	let mut sorted_mounts = config_mounts.iter().collect::<Vec<_>>();
	sorted_mounts.sort_by(|(a, _), (b, _)| a.cmp(b));
	let mut mount_configs = Vec::with_capacity(mount_count);
	for (ordinal, (path, content)) in sorted_mounts.into_iter().enumerate() {
		let (config_id, config_name) = crate::utils::update_config(
			docker,
			&format!("config-{}-{}", id, ordinal),
			HashMap::from([
				(String::from("managed-by"), String::from("patr")),
				(String::from("patr.deploymentId"), id.to_string()),
			]),
			content.to_string(),
		)
		.await?;

		mount_configs.push(TaskSpecContainerSpecConfigs {
			file: Some(TaskSpecContainerSpecFile1 {
				name: Some(path.clone()),
				mode: Some(0o444),
				uid: Some(String::from("0")),
				gid: Some(String::from("0")),
			}),
			config_id: Some(config_id),
			config_name: Some(config_name),
			runtime: None,
		});
	}

	// Build the service spec
	let networks = Some(vec![NetworkAttachmentConfig {
		target: Some(String::from(constants::INGRESS_NETWORK_NAME)),
		aliases: Some(vec![format!("{}.onpatr.local", id)]),
		driver_opts: None,
	}]);

	let service_spec = ServiceSpec {
		name: Some(service_name.clone()),
		labels: Some(HashMap::from([
			(String::from("managed-by"), String::from("patr")),
			(
				String::from("patr.version"),
				String::from(constants::PATR_VERSION),
			),
			(String::from("patr.deploymentId"), id.to_string()),
			(String::from("patr.deploymentName"), name.clone()),
			(
				String::from("patr.minHorizontalScale"),
				min_horizontal_scale.to_string(),
			),
			(
				String::from("patr.maxHorizontalScale"),
				max_horizontal_scale.to_string(),
			),
		])),
		task_template: Some(TaskSpec {
			container_spec: Some(TaskSpecContainerSpec {
				image: Some(image.clone()),
				hostname: Some(format!("{}.onpatr.cloud", id)),
				env: Some(
					environment_variables
						.into_iter()
						.map(|(key, value)| {
							format!(
								"{}={}",
								key,
								match value {
									EnvironmentVariableValue::String(value) => value,
									EnvironmentVariableValue::Secret { from_secret: _ } => todo!(),
								}
							)
						})
						.collect(),
				),
				labels: Some(HashMap::from([
					(String::from("managed-by"), String::from("patr")),
					(
						String::from("patr.version"),
						String::from(constants::PATR_VERSION),
					),
					(String::from("patr.deploymentId"), id.to_string()),
					(String::from("patr.deploymentName"), name.clone()),
				])),
				health_check,
				hosts: Some(vec![format!("host.docker.internal:{host_ip}")]),
				configs: if mount_configs.is_empty() {
					None
				} else {
					Some(mount_configs)
				},
				..Default::default()
			}),
			networks: networks.clone(),
			..Default::default()
		}),
		mode: Some(ServiceSpecMode {
			replicated: Some(ServiceSpecModeReplicated {
				// Use min_horizontal_scale as the initial replica count
				// Autoscaling between min and max would require external tools
				replicas: Some(min_horizontal_scale as i64),
			}),
			..Default::default()
		}),
		networks,
		..Default::default()
	};

	if let Some(service) = existing_service {
		// Update existing service
		let version = service.version.as_ref().and_then(|v| v.index).unwrap_or(0);

		let options = UpdateServiceOptionsBuilder::default()
			.version(version as i32)
			.build();

		docker
			.update_service(&service_name, service_spec, options, registry_auth.clone())
			.await
			.map_err(|err| {
				error!("Error updating service: {:?}", err);
				RunnerError::host(err)
			})?;
		info!("Service updated");
	} else {
		// Create new service
		docker
			.create_service(service_spec, registry_auth)
			.await
			.map_err(|err| {
				error!("Error creating service: {:?}", err);
				RunnerError::host(err)
			})?;
		info!("Service created");
	}

	// Clean up configs whose mount ordinal is past the current mount count
	// (i.e. a mount was removed from this deployment). `update_config`'s own
	// cleanup is scoped per base_name, so it only handles content churn within
	// an ordinal — not a whole ordinal going away.
	let base_name_prefix = format!("config-{}-", id);
	let existing_mount_configs = docker
		.list_configs(Some(ListConfigsOptions {
			filters: Some(HashMap::from([(
				String::from("label"),
				vec![format!("patr.deploymentId={}", id)],
			)])),
		}))
		.await
		.map_err(RunnerError::host)?;

	for config in existing_mount_configs {
		let Some(base) = config
			.spec
			.as_ref()
			.and_then(|spec| spec.labels.as_ref())
			.and_then(|labels| labels.get("patr.configBaseName"))
		else {
			continue;
		};
		let Some(ordinal_str) = base.strip_prefix(&base_name_prefix) else {
			continue;
		};
		let Ok(ordinal) = ordinal_str.parse::<usize>() else {
			continue;
		};
		if ordinal < mount_count {
			continue;
		}

		if let Some(config_id) = config.id.as_ref() &&
			let Err(err) = docker.delete_config(config_id).await
		{
			warn!(
				"Failed to clean up orphaned mount config {} (base={}): {}",
				config_id, base, err
			);
		}
	}

	info!("Updating ingress config for deployment: {}", id);

	let mut config = String::new();
	for (port, _) in ports
		.into_iter()
		.filter(|(_, r#type)| matches!(r#type, ExposedPortType::Http))
	{
		config.push_str(&ingress::generate_config_for_deployment(
			id,
			port.value(),
			settings.data.runner_exposure_type.is_private(),
		));
	}

	// Only create/update the ingress config if there are HTTP ports.
	// Docker rejects configs with 0 bytes of data.
	if !config.is_empty() {
		let config = Base64String::from_string(config);

		crate::utils::update_config(
			docker,
			&format!("ingress-{}", id),
			HashMap::from([
				(String::from("managed-by"), String::from("patr")),
				(String::from("patr.deploymentId"), id.to_string()),
			]),
			config.to_string(),
		)
		.await?;
	}

	Ok(())
}

/// List all running deployments. This will return a stream of deployment IDs.
/// The stream will yield deployment IDs as they are found. If no deployments
/// are found, the stream will be empty.
pub(crate) async fn list_running<'a>(
	DockerRunner { docker, .. }: &DockerRunner,
) -> impl Stream<Item = Uuid> + 'a {
	let Ok(mut services) = docker
		.list_services(Some(
			ListServicesOptionsBuilder::new()
				.filters(&HashMap::from([("label", vec!["patr.deploymentId"])]))
				.build(),
		))
		.await
	else {
		return futures::stream::empty().boxed();
	};

	services.sort_by(|a, b| {
		let a = a
			.spec
			.as_ref()
			.and_then(|spec| spec.labels.as_ref())
			.and_then(|labels| labels.get("patr.deploymentId"))
			.and_then(|value| Uuid::parse_str(value).ok());
		let b = b
			.spec
			.as_ref()
			.and_then(|spec| spec.labels.as_ref())
			.and_then(|labels| labels.get("patr.deploymentId"))
			.and_then(|value| Uuid::parse_str(value).ok());

		a.cmp(&b)
	});

	futures::stream::iter(services.into_iter().filter_map(|service| {
		service
			.spec
			.and_then(|spec| spec.labels)
			.unwrap_or_default()
			.get("patr.deploymentId")
			.and_then(|value| Uuid::parse_str(value).ok())
	}))
	.boxed()
}

/// Delete the deployment with the given ID. This will remove the service
/// if it exists. The deployment's ingress config is not deleted here — it
/// must be removed via [`ingress::delete_deployment_config`] which first
/// unmounts it from Caddy.
pub(crate) async fn delete(
	DockerRunner { docker, .. }: &DockerRunner,
	id: Uuid,
) -> Result<(), RunnerError> {
	let service_name = format!("patr-{}", id);

	// Check if the service exists
	let service = docker
		.list_services(Some(
			ListServicesOptionsBuilder::new()
				.filters(&HashMap::from([("name", vec![service_name.as_str()])]))
				.build(),
		))
		.await
		.map_err(|err| {
			error!("Error listing services: {:?}", err);
			RunnerError::host(err)
		})?
		.into_iter()
		.find(|s| {
			s.spec
				.as_ref()
				.and_then(|spec| spec.name.as_ref())
				.is_some_and(|name| *name == service_name)
		});

	if service.is_some() {
		docker.delete_service(&service_name).await.map_err(|err| {
			error!("Error removing service: {:?}", err);
			RunnerError::host(err)
		})?;
	}

	// Clean up mount configs owned by this deployment. Safe now that the
	// service is gone — no running task references them.
	let mount_configs = docker
		.list_configs(Some(ListConfigsOptions {
			filters: Some(HashMap::from([(
				String::from("label"),
				vec![format!("patr.deploymentId={}", id)],
			)])),
		}))
		.await
		.map_err(RunnerError::host)?;

	for config in mount_configs {
		if let Some(config_id) = config.id.as_ref() &&
			let Err(err) = docker.delete_config(config_id).await
		{
			warn!(
				"Failed to clean up mount config {} for deleted deployment {}: {}",
				config_id, id, err
			);
		}
	}

	Ok(())
}

/// Get the status of the deployment with the given ID.
pub(crate) async fn get_status(
	DockerRunner { docker, .. }: &DockerRunner,
	deployment_id: Uuid,
) -> Result<DeploymentStatus, RunnerError> {
	let status = docker
		.list_services(Some(ListServicesOptions {
			filters: Some(HashMap::from([(
				String::from("label"),
				vec![format!("patr.deploymentId={}", deployment_id)],
			)])),
			status: Some(true),
		}))
		.await
		.ok()
		.and_then(|services| services.into_iter().next())
		.and_then(|service| service.service_status)
		.map(|status| {
			// If the running tasks are less than the desired tasks, consider it as
			// deploying. If they are equal and greater than zero, consider it running.
			if let (Some(running), Some(desired)) = (status.running_tasks, status.desired_tasks) {
				if running == desired {
					DeploymentStatus::Running
				} else if running < desired {
					DeploymentStatus::Deploying
				} else {
					DeploymentStatus::Stopped
				}
			} else {
				DeploymentStatus::Stopped
			}
		})
		.unwrap_or(DeploymentStatus::Stopped);

	Ok(status)
}
