use std::{collections::BTreeMap, pin::pin};

use futures::StreamExt;
use models::api::workspace::deployment::*;
use tokio::time::{Duration, Instant};

use crate::{prelude::*, utils::delayed_future::DelayedFuture};

impl<E> super::Runner<E>
where
	E: RunnerExecutor + Clone + 'static,
{
	/// Reconcile all the deployments that the runner is responsible for. This
	/// function will run the reconciliation for all the deployments that the
	/// runner is responsible for.
	pub(super) async fn reconcile_all_deployments(&mut self) {
		// Reconcile all deployments
		info!("Reconciling all deployments");

		// Update running deployments
		let Ok(mut should_run_deployments) = self.get_all_local_deployments().await else {
			return;
		};

		let mut running_deployments = pin!(self.executor.list_running_deployments().await);

		while let Some(deployment_id) = running_deployments.next().await {
			let deployment = should_run_deployments
				.iter()
				.find(|&&id| deployment_id == id);

			// If the deployment does not exist in the should run list, delete it
			let Some(&deployment_id) = deployment else {
				trace!(
					"Deployment `{}` does not exist in the should run list",
					deployment_id
				);
				info!("Deleting deployment `{}`", deployment_id);

				if let Err(wait_time) = self.executor.delete_deployment(deployment_id).await {
					self.reconciliation_list.push(DelayedFuture::new(
						Instant::now() + wait_time,
						deployment_id,
					));
					self.recheck_next_reconcile_future();
				}
				return;
			};

			// If it does exist, reconcile the deployment and remove it from the should run
			// list
			self.reconcile_deployment(deployment_id).await;
			should_run_deployments.retain(|&id| id != deployment_id);
		}

		// All remaining deployments are the ones that are there in the should run list,
		// but aren't running. So get them up and running
		for deployment_id in should_run_deployments {
			self.reconcile_deployment(deployment_id).await;
		}
	}

	/// Reconcile a specific deployment. This function will run the
	/// reconciliation for a specific deployment (based on the ID)
	pub(super) async fn reconcile_deployment(&mut self, deployment_id: Uuid) {
		trace!("Reconciling deployment `{}`", deployment_id);
		self.reconciliation_list
			.retain(|message| message.value() != &deployment_id);

		let result = 'reconcile: {
			let GetDeploymentInfoResponse {
				deployment,
				running_details,
			} = match self.get_deployment_info(deployment_id).await {
				Ok(response) => response,
				Err(ErrorType::ResourceDoesNotExist) => {
					info!("Deployment `{}` does not exist. Deleting", deployment_id);
					break 'reconcile self.delete_deployment(deployment_id).await;
				}
				Err(err) => {
					debug!(
						"Failed to get deployment info for `{}`: {:?}",
						deployment_id, err
					);
					break 'reconcile Err(Duration::from_secs(5));
				}
			};

			if let Err(err) = self
				.executor
				.upsert_deployment(deployment, running_details)
				.await
			{
				break 'reconcile Err(err);
			}

			Ok(())
		};

		if let Err(wait_time) = result {
			self.reconciliation_list.push(DelayedFuture::new(
				Instant::now() + wait_time,
				deployment_id,
			));
		}

		self.recheck_next_reconcile_future();
	}

	/// Get all the local deployments. This function will get all the local
	/// deployments from the SQLite database.
	async fn get_all_local_deployments(&mut self) -> Result<Vec<Uuid>, ErrorType> {
		let rows = query(
			r#"
			SELECT
				id
			FROM
				deployment
			ORDER BY
				id;
			"#,
		)
		.fetch_all(&self.state.database)
		.await?;

		Ok(rows
			.into_iter()
			.map(|row| row.get::<Uuid, _>("id"))
			.collect())
	}

	/// Get the deployment info. This function will get the deployment info from
	/// the local database if the runner is self-hosted, or from the API if the
	/// runner is managed.
	async fn get_deployment_info(
		&self,
		deployment_id: Uuid,
	) -> Result<GetDeploymentInfoResponse, ErrorType> {
		match &self.state.config.mode {
			RunnerMode::SelfHosted {
				password_pepper: _,
				jwt_secret: _,
			} => {
				let ports = query(
					r#"
					SELECT
						port,
						port_type
					FROM
						deployment_exposed_port
					WHERE
						deployment_id = $1;
					"#,
				)
				.bind(deployment_id)
				.fetch_all(&self.state.database)
				.await?
				.into_iter()
				.map(|row| {
					let port = row.try_get::<u16, _>("port")?;
					let port_type = row.try_get::<ExposedPortType, _>("port_type")?;

					Ok((StringifiedU16::new(port), port_type))
				})
				.collect::<Result<BTreeMap<_, _>, ErrorType>>()?;

				let environment_variables = query(
					r#"
					SELECT
						name,
						value,
						secret_id
					FROM
						deployment_environment_variable
					WHERE
						deployment_id = $1;
					"#,
				)
				.bind(deployment_id)
				.fetch_all(&self.state.database)
				.await?
				.into_iter()
				.map(|env| {
					let name = env.try_get::<String, _>("name")?;
					let value = env
						.try_get::<Option<String>, _>("value")?
						.map(EnvironmentVariableValue::String);

					let secret_id = env
						.try_get::<Option<Uuid>, _>("secret_id")?
						.map(|from_secret| EnvironmentVariableValue::Secret { from_secret });

					let value = match (value, secret_id) {
						(Some(value), None) => Some(value),
						(None, Some(secret)) => Some(secret),
						_ => None,
					}
					.ok_or(ErrorType::server_error(
						"corrupted deployment, cannot find environment variable value",
					))?;

					Ok((name, value))
				})
				.collect::<Result<BTreeMap<_, _>, ErrorType>>()?;

				let config_mounts = query(
					r#"
					SELECT
						path,
						file
					FROM
						deployment_config_mounts
					WHERE
						deployment_id = $1;
					"#,
				)
				.bind(deployment_id)
				.fetch_all(&self.state.database)
				.await?
				.into_iter()
				.map(|row| {
					let path = row.try_get::<String, _>("path")?;
					let file = row.try_get::<Vec<u8>, _>("file").map(Base64String::from)?;

					Ok((path, file))
				})
				.collect::<Result<BTreeMap<_, _>, ErrorType>>()?;

				let volumes = query(
					r#"
					SELECT
						volume_id,
						volume_mount_path
					FROM
						deployment_volume_mount
					WHERE
						deployment_id = $1;
					"#,
				)
				.bind(deployment_id)
				.fetch_all(&self.state.database)
				.await?
				.into_iter()
				.map(|row| {
					let volume_id = row.try_get::<Uuid, _>("volume_id")?;
					let volume_mount_path = row.try_get::<String, _>("volume_mount_path")?;

					Ok((volume_id, volume_mount_path))
				})
				.collect::<Result<BTreeMap<_, _>, ErrorType>>()?;

				query(
					r#"
					SELECT
						id,
						name,
						registry,
						image_name,
						image_tag,
						status,
						min_horizontal_scale,
						max_horizontal_scale,
						machine_type,
						deploy_on_push,
						startup_probe_port,
						startup_probe_path,
						startup_probe_port_type,
						liveness_probe_port,
						liveness_probe_path,
						liveness_probe_port_type,
						current_live_digest
					FROM
						deployment
					WHERE
						id = $1 AND
						deleted IS NULL;
					"#,
				)
				.bind(deployment_id)
				.fetch_optional(&self.state.database)
				.await?
				.map(|row| {
					let deployment_id = row.try_get::<Uuid, _>("id")?;
					let name = row.try_get::<String, _>("name")?;
					let image_tag = row.try_get::<String, _>("image_tag")?;
					let status = row.try_get::<DeploymentStatus, _>("status")?;
					let registry = row.try_get::<String, _>("registry")?;
					let image_name = row.try_get::<String, _>("image_name")?;
					let machine_type = row.try_get::<Uuid, _>("machine_type")?;
					let current_live_digest =
						row.try_get::<Option<String>, _>("current_live_digest")?;

					let deploy_on_push = row.try_get::<bool, _>("deploy_on_push")?;
					let min_horizontal_scale = row.try_get::<u16, _>("min_horizontal_scale")?;
					let max_horizontal_scale = row.try_get::<u16, _>("max_horizontal_scale")?;

					Ok::<_, ErrorType>(GetDeploymentInfoResponse {
						deployment: WithId::new(
							deployment_id,
							Deployment {
								name,
								image_tag,
								status,
								registry: DeploymentRegistry::ExternalRegistry {
									registry,
									image_name,
								},
								// WARN: This is a dummy runner ID, as there is no runner-id in
								// self-hosted PATR
								runner: Uuid::nil(),
								current_live_digest,
								machine_type,
							},
						),
						running_details: DeploymentRunningDetails {
							deploy_on_push,
							min_horizontal_scale,
							max_horizontal_scale,
							ports,
							environment_variables,
							startup_probe: row
								.try_get::<Option<u16>, _>("startup_probe_port")?
								.zip(row.try_get::<Option<String>, _>("startup_probe_path")?)
								.map(|(port, path)| DeploymentProbe { port, path }),
							liveness_probe: row
								.try_get::<Option<u16>, _>("liveness_probe_port")?
								.zip(row.try_get::<Option<String>, _>("liveness_probe_path")?)
								.map(|(port, path)| DeploymentProbe { port, path }),
							config_mounts,
							volumes,
						},
					})
				})
				.ok_or(ErrorType::ResourceDoesNotExist)?
			}
			RunnerMode::Managed {
				workspace_id,
				runner_id: _,
				api_token,
				user_agent,
			} => client::make_request(
				ApiRequest::<GetDeploymentInfoRequest>::builder()
					.path(GetDeploymentInfoPath {
						workspace_id: *workspace_id,
						deployment_id,
					})
					.headers(GetDeploymentInfoRequestHeaders {
						authorization: api_token.clone(),
						user_agent: user_agent.clone(),
					})
					.query(())
					.body(GetDeploymentInfoRequest)
					.build(),
			)
			.await
			.map(|response| response.body)
			.map_err(|err| {
				debug!(
					"Failed to get deployment info for `{}`: {:?}",
					deployment_id, err
				);
				debug!("Retrying in 5 seconds");
				err.body.error
			}),
		}
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

		self.executor.delete_deployment(id).await?;

		Ok(())
	}
}
