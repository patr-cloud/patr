use futures::Stream;
use models::api::workspace::deployment::*;
use sqlx::sqlite::SqliteRow;
use tokio::time::Duration;

use crate::prelude::*;

impl<E> super::Runner<E>
where
	E: RunnerExecutor + Clone + 'static,
{
	/// Resync all the deployments that the runner is responsible for. This
	/// function will sync the deployments that are running with the deployments
	/// that should be running.
	pub(super) async fn resync_all_deployments(&self) -> Result<(), RunnerError> {
		info!("Reconciling all deployments");
		let RunnerMode::Managed {
			workspace_id,
			runner_id,
			api_token,
			user_agent,
		} = self.state.config.mode.clone()
		else {
			// If the runner is running in self-hosted mode, return early. There's nothing
			// to do here
			return Ok(());
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

		Ok(())
	}

	/// Reconcile a specific deployment. This function will run the
	/// reconciliation for a specific deployment (based on the ID)
	pub(super) async fn reconcile_deployment(&self, deployment_id: Uuid) {
		trace!("Reconciling deployment `{}`", deployment_id);
	}

	/// Get all the local deployments. This function will get all the local
	/// deployments from the SQLite database.
	fn get_all_local_deployments(&self) -> impl Stream<Item = Result<SqliteRow, sqlx::Error>> {
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
	}

	/// Delete a deployment. This function will delete a deployment from the
	/// database, and call the executor to delete the deployment.
	async fn delete_deployment(&self, id: Uuid) -> Result<(), Duration> {
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
		.await
		.map_err(|err| {
			debug!("Failed to delete deployment `{}`: {:?}", id, err);
			debug!("Retrying in 5 seconds");
			Duration::from_secs(5)
		})?;

		// self.registry.get(&id).get_or_insert_default().stop().await;

		Ok(())
	}
}
