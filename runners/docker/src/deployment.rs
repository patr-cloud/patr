use std::collections::HashMap;

use bollard::{
	models::ContainerCreateBody,
	query_parameters::{
		CreateContainerOptionsBuilder,
		CreateImageOptionsBuilder,
		ListContainersOptionsBuilder,
		RemoveContainerOptionsBuilder,
		StartContainerOptions,
		StopContainerOptionsBuilder,
	},
	secret::CreateImageInfo,
};
use common::prelude::*;
use futures::{Stream, StreamExt};
use models::api::workspace::deployment::*;

use crate::DockerRunner;

pub(crate) async fn upsert(
	DockerRunner { docker }: &DockerRunner,
	WithId {
		id,
		data:
			Deployment {
				name: _,
				registry,
				image_tag,
				status,
				runner: _,
				machine_type,
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
		volumes,
	}: DeploymentRunningDetails,
) -> Result<(), RunnerError> {
	// Check if the container exists, first.
	let container = docker
		.list_containers(Some(
			ListContainersOptionsBuilder::new()
				.all(true)
				.filters(&HashMap::from([(
					"label",
					vec![format!("patr.deploymentId={}", id)],
				)]))
				.build(),
		))
		.await
		.map_err(|err| {
			error!("Error listing containers: {:?}", err);
			RunnerError::host(err)
		})?
		.into_iter()
		.next();

	if let Some(container) = container {
		docker
			.stop_container(
				container.id.as_deref().unwrap(),
				Some(StopContainerOptionsBuilder::new().t(30).build()),
			)
			.await
			.map_err(|err| {
				error!("Error stopping container: {:?}", err);
				RunnerError::host(err)
			})?;
		docker
			.remove_container(
				container.id.as_deref().unwrap_or_default(),
				Some(
					RemoveContainerOptionsBuilder::new()
						.force(true)
						.v(false)
						.build(),
				),
			)
			.await
			.map_err(|err| {
				error!("Error removing container: {:?}", err);
				RunnerError::host(err)
			})?;
	}

	info!("Pulling latest image...");
	let mut pull_image = docker.create_image(
		Some(
			CreateImageOptionsBuilder::new()
				.from_image(&format!(
					"{}/{}{}{}",
					registry.registry_url(),
					registry.image_name().unwrap(),
					if current_live_digest.is_some() {
						'@'
					} else {
						':'
					},
					current_live_digest.as_deref().unwrap_or(&image_tag)
				))
				.build(),
		),
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

	let container = docker
		.create_container(
			Some(
				CreateContainerOptionsBuilder::new()
					.name(&id.to_string())
					.build(),
			),
			ContainerCreateBody {
				hostname: Some(format!("{}.onpatr.cloud", id)),
				image: Some(format!(
					"{}/{}{}{}",
					registry.registry_url(),
					registry.image_name().unwrap(),
					if current_live_digest.is_some() {
						'@'
					} else {
						':'
					},
					current_live_digest.as_deref().unwrap_or(&image_tag)
				)),
				exposed_ports: Some(
					ports
						.into_iter()
						.map(|(port, port_type)| {
							{
								(
									format!(
										"{}/{}",
										port,
										match port_type {
											ExposedPortType::Tcp | ExposedPortType::Http => "tcp",
											ExposedPortType::Udp => "udp",
										}
									),
									HashMap::<(), ()>::new(),
								)
							}
						})
						.collect(),
				),
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
						.collect::<Vec<_>>(),
				),
				labels: Some(HashMap::from([(
					String::from("patr.deploymentId"),
					id.to_string(),
				)])),
				..Default::default()
			},
		)
		.await
		.map_err(|err| {
			error!("Error creating container: {:?}", err);
			RunnerError::host(err)
		})?;
	info!("Container created");

	docker
		.start_container(&container.id, None::<StartContainerOptions>)
		.await
		.map_err(|err| {
			error!("Error starting container: {:?}", err);
			RunnerError::host(err)
		})?;
	info!("Container started");

	Ok(())
}

pub(crate) async fn list_running<'a>(
	DockerRunner { docker }: &DockerRunner,
) -> impl Stream<Item = Uuid> + 'a {
	let Ok(mut containers) = docker
		.list_containers(Some(
			ListContainersOptionsBuilder::new()
				.all(true)
				.filters(&HashMap::<String, Vec<String>>::new())
				.build(),
		))
		.await
	else {
		return futures::stream::empty().boxed();
	};
	containers.sort_by(|a, b| {
		let a = a.labels.as_ref().and_then(|labels| {
			labels
				.get("patr.deploymentId")
				.and_then(|value| Uuid::parse_str(value).ok())
		});
		let b = b.labels.as_ref().and_then(|labels| {
			labels
				.get("patr.deploymentId")
				.and_then(|value| Uuid::parse_str(value).ok())
		});

		a.cmp(&b)
	});

	futures::stream::iter(containers.into_iter().filter_map(|container| {
		container
			.labels
			.unwrap_or_default()
			.get("patr.deploymentId")
			.and_then(|value| Uuid::parse_str(value).ok())
	}))
	.boxed()
}

pub(crate) async fn delete(
	DockerRunner { docker }: &DockerRunner,
	id: Uuid,
) -> Result<(), RunnerError> {
	// Check if the container exists, first.
	let container = docker
		.list_containers(Some(
			ListContainersOptionsBuilder::new()
				.all(true)
				.filters(&HashMap::from([(
					"label",
					vec![format!("patr.deploymentId={}", id)],
				)]))
				.build(),
		))
		.await
		.map_err(|err| {
			error!("Error listing containers: {:?}", err);
			RunnerError::host(err)
		})?
		.into_iter()
		.next();

	if let Some(container) = container {
		docker
			.remove_container(
				container.id.as_deref().unwrap_or_default(),
				Some(
					RemoveContainerOptionsBuilder::new()
						.force(true)
						.v(false)
						.build(),
				),
			)
			.await
			.map_err(|err| {
				error!("Error removing container: {:?}", err);
				RunnerError::host(err)
			})?;
	}

	Ok(())
}
