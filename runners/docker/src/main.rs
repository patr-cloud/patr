#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::all)]

//! The Docker runner is a service that runs on a machine and listens for
//! incoming WebSocket connections from the Patr API. The runner is responsible
//! for creating, updating, and deleting deployments in the given runner.

/// The configuration for the runner.
mod config;

use std::{collections::HashMap, time::Duration};

use bollard::{
	container::{
		Config,
		CreateContainerOptions,
		ListContainersOptions,
		RemoveContainerOptions,
		StopContainerOptions,
		UpdateContainerOptions,
	},
	secret::{RestartPolicy, ServiceSpec},
	Docker,
};
use common::prelude::*;
use futures::Stream;
use models::api::workspace::deployment::*;

struct DockerRunner {
	docker: Docker,
}

impl RunnerExecutor for DockerRunner {
	type Settings<'s> = config::RunnerSettings;

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
					runner,
					machine_type,
					current_live_digest,
				},
		}: WithId<Deployment>,
		DeploymentRunningDetails {
			deploy_on_push,
			min_horizontal_scale,
			max_horizontal_scale,
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
						"{}/{}",
						registry.registry_url(),
						registry.image_name().unwrap()
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
										EnvironmentVariableValue::Secret { from_secret } => todo!(),
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

		self.docker
			.start_container::<String>(&container.id, None)
			.await
			.map_err(|err| {
				error!("Error starting container: {:?}", err);
				Duration::from_secs(5)
			})?;

		Ok(())
	}

	fn list_running_deployments<'a>(&self) -> impl Stream<Item = Uuid> + 'a {
		futures::stream::empty()
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
	Runner::new(DockerRunner {
		docker: Docker::connect_with_local_defaults().unwrap(),
	})
	.expect("unable to construct runner")
	.run()
	.await;
}
