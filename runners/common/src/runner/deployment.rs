use std::{cmp::Ordering, collections::BTreeMap};

use futures::{Stream, StreamExt};
use models::api::workspace::deployment::*;
use tokio_stream as stream;

use crate::{prelude::*, utils::resource_executor::ResourceExecutorTask};

impl<E> super::Runner<E>
where
	E: RunnerExecutor + Send + 'static,
{
	/// Resync all the deployments that the runner is responsible for. This
	/// function will sync the local database with the upstream API, making sure
	/// both are in sync.
	#[instrument(skip(self, api_token))]
	pub(super) async fn resync_all_deployments_with_upstream(
		&self,
		workspace_id: Uuid,
		runner_id: Uuid,
		api_token: &BearerToken,
		user_agent: &UserAgent,
	) -> Result<(), RunnerError> {
		info!("Resync all deployments with upstream API");
		// Update running deployments
		let mut transaction = self.state.database.begin().await?;

		trace!("Deleting all deployment volume mounts from local database");
		query(
			r#"
			DELETE FROM deployment_volume_mount;
			"#,
		)
		.execute(&mut *transaction)
		.await?;

		trace!("Deleting all deployment deploy history from local database");
		query(
			r#"
			DELETE FROM deployment_deploy_history;
			"#,
		)
		.execute(&mut *transaction)
		.await?;

		trace!("Deleting all deployment config mounts from local database");
		query(
			r#"
			DELETE FROM deployment_config_mounts;
			"#,
		)
		.execute(&mut *transaction)
		.await?;

		trace!("Deleting all deployment exposed ports from local database");
		query(
			r#"
			DELETE FROM deployment_exposed_port;
			"#,
		)
		.execute(&mut *transaction)
		.await?;

		trace!("Deleting all deployment environment variables from local database");
		query(
			r#"
			DELETE FROM deployment_environment_variable;
			"#,
		)
		.execute(&mut *transaction)
		.await?;

		trace!("Deleting all deployments from local database");
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
			.with_cancel_check()
			.await?
			.map_err(|err| err.body.error)?;

			if page * Paginated::DEFAULT_PAGE_SIZE >= response.headers.total_count.0 {
				break;
			}

			for deployment in response
				.body
				.deployments
				.into_iter()
				.filter(|deployment| deployment.runner == runner_id)
			{
				let deployment_id = deployment.id;

				let GetDeploymentInfoResponse {
					deployment,
					running_details,
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
				.with_cancel_check()
				.await?
				.map_err(|err| err.body.error)?
				.body;

				self.create_deployment_in_database(&mut transaction, deployment, running_details)
					.await?;
			}

			page += 1;
		}

		transaction.commit().await?;

		Ok(())
	}

	/// Create a deployment. This function will create a deployment in the local
	/// database. The executor will have to handle the actual creation of the
	/// deployment.
	#[instrument(skip(self, connection))]
	pub(super) async fn create_deployment_in_database(
		&self,
		connection: &mut DatabaseConnection,
		WithId {
			id: deployment_id,
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
	) -> Result<(), RunnerError> {
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
		.execute(&mut *connection)
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
			.execute(&mut *connection)
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
			.execute(&mut *connection)
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
			.execute(&mut *connection)
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
			.execute(&mut *connection)
			.await?;
		}
		trace!("Inserted volume mounts for deployment");

		Ok(())
	}

	/// Delete a deployment. This function will delete a deployment from the
	/// database. The executor will have to handle the actual deletion of the
	/// deployment.
	#[instrument(skip(self, connection))]
	pub(super) async fn delete_deployment_in_database(
		&self,
		connection: &mut DatabaseConnection,
		deployment_id: Uuid,
	) -> Result<(), RunnerError> {
		query(
			r#"
			DELETE FROM
				deployment_volume_mount
			WHERE
				deployment_id = $1;
			"#,
		)
		.bind(deployment_id)
		.execute(&mut *connection)
		.await?;

		query(
			r#"
			DELETE FROM
				deployment_deploy_history
			WHERE
				deployment_id = $1;
			"#,
		)
		.bind(deployment_id)
		.execute(&mut *connection)
		.await?;

		query(
			r#"
			DELETE FROM
				deployment_config_mounts
			WHERE
				deployment_id = $1;
			"#,
		)
		.bind(deployment_id)
		.execute(&mut *connection)
		.await?;

		query(
			r#"
			DELETE FROM
				deployment_exposed_port
			WHERE
				deployment_id = $1;
			"#,
		)
		.bind(deployment_id)
		.execute(&mut *connection)
		.await?;

		query(
			r#"
			DELETE FROM
				deployment_environment_variable
			WHERE
				deployment_id = $1;
			"#,
		)
		.bind(deployment_id)
		.execute(&mut *connection)
		.await?;

		query(
			r#"
			DELETE FROM
				deployment
			WHERE
				id = $1;
			"#,
		)
		.bind(deployment_id)
		.execute(&mut *connection)
		.await?;

		Ok(())
	}

	/// Reconcile all deployments. This will check the database for all running
	/// deployments and make sure that they are running on the host. If a
	/// deployment is not running on the host, it will be started. If a
	/// deployment is running on the host but is not in the database, it will be
	/// stopped.
	#[instrument(skip(self))]
	pub(super) async fn reconcile_deployments(&self) -> Result<(), RunnerError> {
		let mut running_deployments =
			stream::iter(self.registry.iter().map(|item| item.value().resource_id()));
		let mut database_deployments = self.get_all_local_deployment_ids().await;

		let mut current_running_deployment = running_deployments.next().with_cancel_check().await?;
		let mut current_database_deployment =
			database_deployments.next().with_cancel_check().await?;

		// Okay, so the plan is simple:
		//
		// Iterate over the list of deployment IDs that are currently running on the
		// host, sorted by the ID (we'll call this running_deployments) and the list
		// of deployment IDs that are supposed to be running on the runner (we'll call
		// this database_deployments), again sorted by ID. Iterate over both one
		// element at a time and compare each element. If the running deployments is
		// less than the database deployment, then the current database deployment ID is
		// "ahead", of the running deployment ID. Which means that the element that is
		// in the running list is not present in the db, since if it was, it would be
		// here (because both lists are sorted). In this case, we delete the running
		// deployment because according to the database, it's not supposed to be
		// running. If the running deployment ID is greater than the database deployment
		// ID, then the database deployment ID is "behind" the running deployment ID. In
		// this case, we create the deployment in the database because according to the
		// database, it should be running. If the two are equal, then we make sure that
		// the task for the deployment is running and move on to the next element in
		// both lists. We continue this process until both lists are exhausted.
		//
		// If one of the list exhausts before the other, then:
		// - If the running deployments list is exhausted, then the remaining elements
		//   in the database deployments list should all be created.
		// - If the database deployments list is exhausted, then the remaining elements
		//   in the running deployments list should all be deleted.

		loop {
			match (current_running_deployment, current_database_deployment) {
				(Some(running_deployment), Some(Ok(database_deployment))) => {
					match running_deployment.cmp(&database_deployment) {
						Ordering::Less => {
							// The running deployment is not in the database. We
							// need to delete it
							self.delete_running_deployment(running_deployment).await?;

							current_running_deployment =
								running_deployments.next().with_cancel_check().await?;
							current_database_deployment = Some(Ok(database_deployment));
						}
						Ordering::Greater => {
							// The database deployment is not running. We need to
							// create it
							self.upsert_running_deployment(database_deployment).await?;

							current_database_deployment =
								database_deployments.next().with_cancel_check().await?;
						}
						Ordering::Equal => {
							current_running_deployment =
								running_deployments.next().with_cancel_check().await?;
							current_database_deployment =
								database_deployments.next().with_cancel_check().await?;
						}
					}
				}
				(Some(running_deployment), None) => {
					// The database is exhausted. We need to delete the running
					// deployment
					self.delete_running_deployment(running_deployment).await?;

					current_database_deployment = None;
					current_running_deployment =
						running_deployments.next().with_cancel_check().await?;
				}
				(None, Some(Ok(database_deployment))) => {
					// The running deployments are exhausted. Create the
					// deployment that is in the database
					self.upsert_running_deployment(database_deployment).await?;

					current_database_deployment =
						database_deployments.next().with_cancel_check().await?;
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

	/// Get all the local deployments. This function will get all the local
	/// deployments from the SQLite database.
	#[instrument(skip(self))]
	pub(super) async fn get_all_local_deployment_ids(
		&self,
	) -> impl Stream<Item = Result<Uuid, RunnerError>> {
		query(
			r#"
			SELECT
				id
			FROM
				deployment
			WHERE
				status = 'running'
			ORDER BY
				id;
			"#,
		)
		.fetch(&self.state.database)
		.map(|row| row.map(|row| row.get::<Uuid, _>("id")).map_err(Into::into))
	}

	pub(super) async fn get_local_deployment_info(
		&self,
		deployment_id: Uuid,
	) -> Result<(Deployment, DeploymentRunningDetails), RunnerError> {
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

		let row = query(
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
		.fetch_one(&self.state.database)
		.await
		.map_err(|err| match err {
			sqlx::Error::RowNotFound => ErrorType::ResourceDoesNotExist,
			err => err.into(),
		})?;

		let name = row.try_get::<String, _>("name")?;
		let image_tag = row.try_get::<String, _>("image_tag")?;
		let status = row.try_get::<DeploymentStatus, _>("status")?;
		let registry = row.try_get::<String, _>("registry")?;
		let image_name = row.try_get::<String, _>("image_name")?;
		let machine_type = row.try_get::<Uuid, _>("machine_type")?;
		let current_live_digest = row.try_get::<Option<String>, _>("current_live_digest")?;

		let deploy_on_push = row.try_get::<bool, _>("deploy_on_push")?;
		let min_horizontal_scale = row.try_get::<u16, _>("min_horizontal_scale")?;
		let max_horizontal_scale = row.try_get::<u16, _>("max_horizontal_scale")?;

		let startup_probe = row
			.try_get::<Option<u16>, _>("startup_probe_port")?
			.zip(row.try_get::<Option<String>, _>("startup_probe_path")?)
			.map(|(port, path)| DeploymentProbe { port, path });

		let liveness_probe = row
			.try_get::<Option<u16>, _>("liveness_probe_port")?
			.zip(row.try_get::<Option<String>, _>("liveness_probe_path")?)
			.map(|(port, path)| DeploymentProbe { port, path });

		Ok((
			Deployment {
				name,
				image_tag,
				status,
				registry: DeploymentRegistry::ExternalRegistry {
					registry,
					image_name,
				},
				// WARN: This is a dummy runner ID, as there is no runner-id in self-hosted PATR
				runner: Uuid::nil(),
				current_live_digest,
				machine_type,
			},
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
		))
	}

	/// Delete a running deployment. This function will delete a running
	/// deployment from the host.
	#[instrument(skip(self))]
	pub(super) async fn delete_running_deployment(
		&self,
		deployment_id: Uuid,
	) -> Result<(), RunnerError> {
		let Some(task) = self.registry.get(&deployment_id) else {
			return Ok(());
		};

		task.stop().await?;

		Ok(())
	}

	/// Upsert a deployment. This function will create a deployment on the host.
	/// This runner executor task will be responsible for updating the database
	/// with the status of the deployment.
	#[instrument(skip(self))]
	pub(super) async fn upsert_running_deployment(
		&self,
		deployment_id: Uuid,
	) -> Result<(), RunnerError> {
		self.registry
			.entry(deployment_id)
			.or_insert_with(|| {
				// Create the task for the deployment
				ResourceExecutorTask::new_deployment(deployment_id, self.state.clone())
			})
			.value_mut()
			.ensure_running()
	}
}
