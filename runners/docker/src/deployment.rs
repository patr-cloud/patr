use std::collections::HashMap;

use bollard::{
	models::{
		ConfigSpec,
		HealthConfig,
		NetworkAttachmentConfig,
		ServiceSpec,
		ServiceSpecMode,
		ServiceSpecModeReplicated,
		TaskSpec,
		TaskSpecContainerSpec,
	},
	query_parameters::{
		CreateImageOptionsBuilder,
		ListConfigsOptions,
		ListServicesOptions,
		ListServicesOptionsBuilder,
		UpdateConfigOptionsBuilder,
		UpdateServiceOptionsBuilder,
	},
	secret::CreateImageInfo,
};
use futures::{Stream, StreamExt};
use models::api::workspace::deployment::*;

use crate::prelude::*;

/// Upsert (create or update) a deployment. This will create the service if
/// it does not exist, or update the service if it does exist.
pub(crate) async fn upsert(
	DockerRunner { docker, .. }: &DockerRunner,
	WithId {
		id,
		data:
			Deployment {
				name: _,
				registry,
				image_tag,
				status: _, // TODO handle paused deployments
				runner: _,
				machine_type: _, // TODO
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
		config_mounts: _, // TODO
		volumes: _,       // TODO
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

	info!("Pulling latest image...");
	let mut pull_image = docker.create_image(
		Some(CreateImageOptionsBuilder::new().from_image(&image).build()),
		None,
		None,
	);
	while let Some(result) = pull_image.next().await {
		match result {
			Ok(CreateImageInfo {
				status: Some(status),
				..
			}) => {
				trace!("Image pull status: {}", status);
			}
			Err(err) => warn!("Unable to pull image: {}", err),
			_ => (),
		}
	}
	info!("Image updated");

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

	// Build the service spec
	let service_spec = ServiceSpec {
		name: Some(service_name.clone()),
		labels: Some(HashMap::from([
			(String::from("patr.deploymentId"), id.to_string()),
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
				labels: Some(HashMap::from([(
					String::from("patr.deploymentId"),
					id.to_string(),
				)])),
				health_check,
				..Default::default()
			}),
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
		networks: Some(vec![NetworkAttachmentConfig {
			target: Some(String::from(constants::INGRESS_NETWORK_NAME)),
			aliases: Some(vec![
				format!("{}.onpatr.cloud", id),
				format!("patr-{}", id),
				format!("{}.onpatr.local", id),
			]),
			driver_opts: None,
		}]),
		..Default::default()
	};

	if let Some(service) = existing_service {
		// Update existing service
		let version = service.version.as_ref().and_then(|v| v.index).unwrap_or(0);

		let options = UpdateServiceOptionsBuilder::default()
			.version(version as i32)
			.build();

		docker
			.update_service(&service_name, service_spec, options, None)
			.await
			.map_err(|err| {
				error!("Error updating service: {:?}", err);
				RunnerError::host(err)
			})?;
		info!("Service updated");
	} else {
		// Create new service
		docker
			.create_service(service_spec, None)
			.await
			.map_err(|err| {
				error!("Error creating service: {:?}", err);
				RunnerError::host(err)
			})?;
		info!("Service created");
	}

	info!("Updating ingress config for deployment: {}", id);

	let mut config = String::new();
	for (port, _) in ports
		.into_iter()
		.filter(|(_, r#type)| matches!(r#type, ExposedPortType::Http))
	{
		config.push_str(&ingress::generate_config_for_deployment(id, port.value()));
	}

	let config = Base64String::from_string(config);

	let config_spec = ConfigSpec {
		name: Some(format!("ingress-{}", id)),
		labels: Some(HashMap::from([(
			String::from("patr.deploymentId"),
			id.to_string(),
		)])),
		data: Some(config.to_string()),
		templating: None,
	};

	if let Some((config_id, index)) = docker
		.list_configs(Some(ListConfigsOptions {
			filters: Some(HashMap::from([(
				String::from("label"),
				vec![format!("patr.deploymentId={}", id)],
			)])),
		}))
		.await
		.map_err(RunnerError::host)?
		.into_iter()
		.next()
		.and_then(|config| Some((config.id?, config.version?.index?)))
	{
		let config_id = config_id;
		trace!(
			"Config exists for deployment {}, updating: {}",
			id, config_id
		);
		docker
			.update_config(
				&config_id,
				config_spec,
				UpdateConfigOptionsBuilder::default()
					.version(index as i64)
					.build(),
			)
			.await
			.map_err(RunnerError::host)?;
	} else {
		trace!("Creating new config for deployment: {}", id);
		docker
			.create_config(config_spec)
			.await
			.map_err(RunnerError::host)?;
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
/// if it exists.
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
