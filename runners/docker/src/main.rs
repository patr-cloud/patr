#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::all)]

//! The Docker runner is a service that runs on a machine and listens for
//! incoming WebSocket connections from the Patr API. The runner is responsible
//! for creating, updating, and deleting deployments in the given runner.

use std::{collections::HashMap, time::Duration};

use bollard::{
	container::{
		Config,
		CreateContainerOptions,
		ListContainersOptions,
		RemoveContainerOptions,
		StopContainerOptions,
	},
	image::CreateImageOptions,
	secret::CreateImageInfo,
	Docker,
};
use common::prelude::*;
use futures::{Stream, StreamExt};
use models::api::workspace::deployment::*;
use serde::{Deserialize, Serialize};

/// The configuration for the runner.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerSettings {}

/// A Patr runner that uses Docker to run deployments.
#[derive(Debug, Clone)]
struct DockerRunner {
	/// The [`Docker`] client.
	docker: Docker,
}

impl RunnerExecutor for DockerRunner {
	type Settings = DockerSettings;

	const RUNNER_INTERNAL_NAME: &'static str = env!("CARGO_CRATE_NAME");

	async fn create(_: &RunnerSettings<Self::Settings>) -> Self {
		let docker = Docker::connect_with_local_defaults().unwrap();
		Self { docker }
	}

	#[allow(unused_variables)]
	async fn upsert_deployment(
		&self,
		WithId {
			id,
			data:
				Deployment {
					name,
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
			min_horizontal_scale: _,
			max_horizontal_scale: _,
			ports,
			environment_variables,
			startup_probe,
			liveness_probe,
			config_mounts,
			volumes,
		}: DeploymentRunningDetails,
	) -> Result<(), Duration> {
		// Check if the container exists, first.
		let container = self
			.docker
			.list_containers(Some(ListContainersOptions {
				filters: HashMap::from([(
					String::from("label"),
					vec![format!("patr.deploymentId={}", id)],
				)]),
				..Default::default()
			}))
			.await
			.map_err(|err| {
				error!("Error listing containers: {:?}", err);
				Duration::from_secs(5)
			})?
			.into_iter()
			.next();

		if let Some(container) = container {
			self.docker
				.stop_container(
					container.id.as_deref().unwrap(),
					Some(StopContainerOptions { t: 30 }),
				)
				.await
				.map_err(|err| {
					error!("Error stopping container: {:?}", err);
					Duration::from_secs(5)
				})?;
			self.docker
				.remove_container(
					container.id.as_deref().unwrap_or_default(),
					Some(RemoveContainerOptions {
						force: true,
						v: false,
						..Default::default()
					}),
				)
				.await
				.map_err(|err| {
					error!("Error removing container: {:?}", err);
					Duration::from_secs(5)
				})?;
		}

		info!("Pulling latest image...");
		let mut pull_image = self.docker.create_image(
			Some(CreateImageOptions {
				from_image: format!(
					"{}/{}{}",
					registry.registry_url(),
					registry.image_name().unwrap(),
					if let Some(ref digest) = current_live_digest {
						format!("@{}", digest)
					} else {
						format!(":{}", image_tag)
					}
				),
				..Default::default()
			}),
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

		let container = self
			.docker
			.create_container(
				Some(CreateContainerOptions {
					name: name.clone(),
					..Default::default()
				}),
				Config {
					hostname: Some(format!("{}.onpatr.cloud", id)),
					image: Some(format!(
						"{}/{}{}",
						registry.registry_url(),
						registry.image_name().unwrap(),
						if let Some(digest) = current_live_digest {
							format!("@{}", digest)
						} else {
							format!(":{}", image_tag)
						}
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
												ExposedPortType::Tcp => "tcp",
												ExposedPortType::Udp => "udp",
												ExposedPortType::Http => "tcp",
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
										EnvironmentVariableValue::Secret { from_secret: _ } =>
											todo!(),
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
				Duration::from_secs(5)
			})?;
		info!("Container created");

		self.docker
			.start_container::<String>(&container.id, None)
			.await
			.map_err(|err| {
				error!("Error starting container: {:?}", err);
				Duration::from_secs(5)
			})?;
		info!("Container started");

		Ok(())
	}

	async fn list_running_deployments<'a>(&self) -> impl Stream<Item = Uuid> + 'a {
		let Ok(mut containers) = self
			.docker
			.list_containers(Some(ListContainersOptions::<String> {
				filters: HashMap::new(),
				..Default::default()
			}))
			.await
			.map_err(|err| {
				error!("Error listing containers: {:?}", err);
				Duration::from_secs(5)
			})
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

	async fn delete_deployment(&self, id: Uuid) -> Result<(), Duration> {
		// Check if the container exists, first.
		let container = self
			.docker
			.list_containers(Some(ListContainersOptions {
				filters: HashMap::from([(
					String::from("label"),
					vec![format!("patr.deploymentId={}", id)],
				)]),
				..Default::default()
			}))
			.await
			.map_err(|err| {
				error!("Error listing containers: {:?}", err);
				Duration::from_secs(5)
			})?
			.into_iter()
			.next();

		if let Some(container) = container {
			self.docker
				.remove_container(
					container.id.as_deref().unwrap_or_default(),
					Some(RemoveContainerOptions {
						force: true,
						v: false,
						..Default::default()
					}),
				)
				.await
				.map_err(|err| {
					error!("Error removing container: {:?}", err);
					Duration::from_secs(5)
				})?;
		}
		Ok(())
	}
}

#[tokio::main]
async fn main() {
	Runner::<DockerRunner>::run().await;
}
