use std::cmp::Ordering;

use futures::{stream, Stream, StreamExt};
use models::{api::workspace::deployment::*, rbac::ResourceType};

use crate::{prelude::*, utils::resource_executor::ResourceExecutorTask};

impl<E> super::Runner<E>
where
	E: RunnerExecutor + Clone + 'static,
{
	/// Resync all the deployments that the runner is responsible for. This
	/// function will sync the deployments that are running with the deployments
	/// that should be running.
	pub(super) async fn resync_all_deployments(&self) -> Result<(), RunnerError> {
		info!("Reconciling all deployments");
		'update_db: {
			let RunnerMode::Managed {
				workspace_id,
				runner_id,
				api_token,
				user_agent,
			} = self.state.config.mode.clone()
			else {
				// If the runner is running in self-hosted mode, return early. There's nothing
				// to do here
				break 'update_db;
			};

			// Update running deployments
			let mut transaction = self.state.database.begin().await?;

			query(
				r#"
				DELETE FROM deployment_deploy_history;
				"#,
			)
			.execute(&mut *transaction)
			.await?;

			query(
				r#"
				DELETE FROM deployment_config_mounts;
				"#,
			)
			.execute(&mut *transaction)
			.await?;

			query(
				r#"
				DELETE FROM deployment_exposed_port;
				"#,
			)
			.execute(&mut *transaction)
			.await?;

			query(
				r#"
				DELETE FROM deployment_environment_variable;
				"#,
			)
			.execute(&mut *transaction)
			.await?;

			query(
				r#"
				DELETE FROM deployment;
				"#,
			)
			.execute(&mut *transaction)
			.await?;

			let mut page = 0;

			loop {
				let response = client::make_request(
					ApiRequest::<ListDeploymentRequest>::builder()
						.path(ListDeploymentPath { workspace_id })
						.query(Paginated {
							data: (),
							count: Paginated::DEFAULT_PAGE_SIZE,
							page,
						})
						.headers(ListDeploymentRequestHeaders {
							authorization: api_token.clone(),
							user_agent: user_agent.clone(),
						})
						.body(ListDeploymentRequest)
						.build(),
				)
				.await
				.map_err(|err| err.body.error)?;

				if page * Paginated::DEFAULT_PAGE_SIZE >= response.headers.total_count.0 {
					break;
				}

				for deployment in response.body.deployments {
					let deployment_id = deployment.id;
					let GetDeploymentInfoResponse {
						deployment:
							WithId {
								id: _,
								data:
									Deployment {
										name,
										registry,
										image_tag,
										runner: _,
										machine_type,
										status,
										current_live_digest,
									},
							},
						running_details:
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
							},
					} = client::make_request(
						ApiRequest::<GetDeploymentInfoRequest>::builder()
							.path(GetDeploymentInfoPath {
								workspace_id,
								deployment_id,
							})
							.query(())
							.headers(GetDeploymentInfoRequestHeaders {
								authorization: api_token.clone(),
								user_agent: user_agent.clone(),
							})
							.body(GetDeploymentInfoRequest)
							.build(),
					)
					.await
					.map_err(|err| err.body.error)?
					.body;

					query(
						r#"
						INSERT INTO
							deployment(
								id,
								name,
								registry,
								image_name,
								image_tag,
								status,
								machine_type,
								min_horizontal_scale,
								max_horizontal_scale,
								deploy_on_push,
								startup_probe_port,
								startup_probe_path,
								startup_probe_port_type,
								liveness_probe_port,
								liveness_probe_path,
								liveness_probe_port_type,
								current_live_digest,
								deleted
							)
						VALUES
							(
								$1,
								$2,
								$3,
								$4,
								$5,
								$6,
								$7,
								$8,
								$9,
								$10,
								$11,
								$12,
								$13,
								$14,
								$15,
								$16,
								$17,
								NULL
							);
						"#,
					)
					.bind(deployment_id)
					.bind(name.to_string())
					.bind(registry.registry_url())
					.bind(registry.image_name())
					.bind(image_tag.to_string())
					.bind(status)
					.bind(machine_type)
					.bind(min_horizontal_scale)
					.bind(max_horizontal_scale)
					.bind(deploy_on_push)
					.bind(startup_probe.as_ref().map(|probe| probe.port))
					.bind(startup_probe.as_ref().map(|probe| probe.path.as_str()))
					.bind(startup_probe.as_ref().map(|_| ExposedPortType::Http))
					.bind(liveness_probe.as_ref().map(|probe| probe.port))
					.bind(liveness_probe.as_ref().map(|probe| probe.path.as_str()))
					.bind(liveness_probe.as_ref().map(|_| ExposedPortType::Http))
					.bind(current_live_digest)
					.execute(&mut *transaction)
					.await?;

					trace!("Created deployment with ID: {}", deployment_id);

					for (port, port_type) in &ports {
						query(
							r#"
							INSERT INTO
								deployment_exposed_port(
									deployment_id,
									port,
									port_type
								)
							VALUES
								(
									$1,
									$2,
									$3
								);
							"#,
						)
						.bind(deployment_id)
						.bind(port.value())
						.bind(port_type)
						.execute(&mut *transaction)
						.await?;
					}

					trace!("Inserted exposed ports for deployment");

					for (name, value) in &environment_variables {
						query(
							r#"
							INSERT INTO
								deployment_environment_variable(
									deployment_id,
									name,
									value,
									secret_id
								)
							VALUES
								(
									$1,
									$2,
									$3,
									$4
								);
							"#,
						)
						.bind(deployment_id)
						.bind(name)
						.bind(value.value())
						.bind(value.secret_id())
						.execute(&mut *transaction)
						.await?;
					}

					trace!("Inserted environment variables for deployment");

					for (path, file) in &config_mounts {
						query(
							r#"
							INSERT INTO
								deployment_config_mounts(
									deployment_id,
									path,
									file
								)
							VALUES
								(
									$1,
									$2,
									$3
								);
							"#,
						)
						.bind(deployment_id)
						.bind(path)
						.bind(file.to_vec())
						.execute(&mut *transaction)
						.await?;
					}

					trace!("Inserted config mounts for deployment");

					for (volume_id, mount_path) in &volumes {
						query(
							r#"
							INSERT INTO 
								deployment_volume_mount(
									deployment_id,
									volume_id,
									volume_mount_path
								)
							VALUES
								(
									$1,
									$2,
									$3
								);
							"#,
						)
						.bind(deployment_id)
						.bind(volume_id)
						.bind(mount_path)
						.execute(&mut *transaction)
						.await?;
					}

					trace!("Inserted volume mounts for deployment");
				}

				page += 1;
			}

			transaction.commit().await?;
		}

		let mut running_deployments =
			stream::iter(self.registry.iter().map(|item| item.value().resource_id()));
		let mut database_deployments = self.get_all_local_deployment_ids().await;

		let mut current_running_deployment = running_deployments.next().await;
		let mut current_database_deployment = database_deployments.next().await;

		loop {
			match (current_running_deployment, current_database_deployment) {
				(Some(running_deployment), Some(Ok(database_deployment))) => {
					match running_deployment.cmp(&database_deployment) {
						Ordering::Less => {
							// The running deployment is not in the database. We
							// need to delete it
							self.delete_deployment(running_deployment).await?;

							current_running_deployment = running_deployments.next().await;
							current_database_deployment = Some(Ok(database_deployment));
						}
						Ordering::Greater => {
							// The database deployment is not running. We need to
							// create it
							self.create_deployment(database_deployment).await?;

							current_database_deployment = database_deployments.next().await;
						}
						Ordering::Equal => {
							current_running_deployment = running_deployments.next().await;
							current_database_deployment = database_deployments.next().await;
						}
					}
				}
				(Some(running_deployment), None) => {
					// The database is exhausted. We need to delete the running
					// deployment
					self.delete_deployment(running_deployment).await?;

					current_database_deployment = None;
					current_running_deployment = running_deployments.next().await;
				}
				(None, Some(Ok(database_deployment))) => {
					// The running deployments are exhausted. Create the
					// deployment that is in the database
					self.create_deployment(database_deployment).await?;

					current_database_deployment = database_deployments.next().await;
				}
				(_, Some(Err(err))) => {
					// There was an error fetching the database deployment. We
					// should retry or exit
					return Err(Into::into(err));
				}
				(None, None) => {
					// Both are exhausted. We're done
					break;
				}
			}
		}

		Ok(())
	}

	async fn create_deployment(&self, deployment_id: Uuid) -> Result<(), RunnerError> {
		self.registry.insert(
			deployment_id,
			ResourceExecutorTask::new(deployment_id, ResourceType::Deployment, &self.state),
		);

		Ok(())
	}

	/// Get all the local deployments. This function will get all the local
	/// deployments from the SQLite database.
	async fn get_all_local_deployment_ids(&self) -> impl Stream<Item = Result<Uuid, sqlx::Error>> {
		query(
			r#"
				SELECT
					id
				FROM
					deployment
				ORDER BY
					id;
				"#,
		)
		.fetch(&self.state.database)
		.map(|row| row.map(|row| row.get::<Uuid, _>("id")))
	}

	/// Delete a deployment. This function will delete a deployment from the
	/// database, and call the executor to delete the deployment.
	async fn delete_deployment(&self, id: Uuid) -> Result<(), RunnerError> {
		query(
			r#"
			DELETE FROM
				deployment_deploy_history
			WHERE
				deployment_id = $1;
			"#,
		)
		.bind(id)
		.execute(&self.state.database)
		.await?;

		query(
			r#"
			DELETE FROM
				deployment_config_mounts
			WHERE
				deployment_id = $1;
			"#,
		)
		.bind(id)
		.execute(&self.state.database)
		.await?;

		query(
			r#"
			DELETE FROM
				deployment_exposed_port
			WHERE
				deployment_id = $1;
			"#,
		)
		.bind(id)
		.execute(&self.state.database)
		.await?;

		query(
			r#"
			DELETE FROM
				deployment_environment_variable
			WHERE
				deployment_id = $1;
			"#,
		)
		.bind(id)
		.execute(&self.state.database)
		.await?;

		query(
			r#"
			DELETE FROM
				deployment
			WHERE
				id = $1;
			"#,
		)
		.bind(id)
		.execute(&self.state.database)
		.await?;

		if let Some((_, item)) = self.registry.remove(&id) {
			item.stop().await;
		}

		Ok(())
	}
}
