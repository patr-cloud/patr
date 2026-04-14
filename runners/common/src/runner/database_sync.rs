use std::pin::pin;

use futures::{
	SinkExt,
	StreamExt,
	future::{self, Either},
};
use models::api::workspace::runner::*;
use tokio::{
	sync::mpsc,
	time::{self, Duration},
};

use crate::prelude::*;

impl<E> super::Runner<E>
where
	E: RunnerExecutor + Send + Sync + 'static,
{
	/// Sync the local database with the upstream APIs. This function will
	/// connect to the server and listen for messages from the server. It will
	/// notify the runner to reconcile the resources that are changed on the
	/// server. This function will only run if the runner is running in managed
	/// mode. This function will exit if the exit signal is received.
	#[instrument(skip(self, receiver))]
	pub(super) async fn sync_local_database(
		&self,
		mut receiver: mpsc::UnboundedReceiver<ExecutorStatusUpdate>,
	) -> Result<!, RunnerError> {
		let RunnerMode::Managed {
			workspace_id,
			runner_id,
			api_token,
			user_agent,
		} = self.state.config.mode.clone()
		else {
			// If the runner is running in self-hosted mode, return early. The run function
			// uses a join of all the futures so early return here will not stop the runner
			// from running
			debug!("Runner is running in self-hosted mode. Skipping sync");
			return Err(RunnerError::Unsupported);
		};

		info!("Syncing local database with upstream APIs");

		info!("Connecting to the server");
		// Connect to the server infinitely until the exit signal is received
		'main: loop {
			let response = client::stream_request(
				ApiRequest::<StreamRunnerDataForWorkspaceRequest>::builder()
					.path(StreamRunnerDataForWorkspacePath {
						workspace_id,
						runner_id,
					})
					.headers(StreamRunnerDataForWorkspaceRequestHeaders {
						authorization: api_token.clone(),
						user_agent: user_agent.clone(),
					})
					.build(),
			)
			.with_cancel_check()
			.await?;

			// Clear any queued messages in the receiver to avoid processing stale updates
			// TODO: These messages will be handled in the full reconcile with the server
			let queued_messages = receiver.len();
			receiver.recv_many(&mut vec![], queued_messages).await;

			let Ok(stream) = response
				.inspect_err(|err| {
					error!("Failed to connect to the server: {:?}", err);
					error!("Retrying in 5 second");
				})
				.map_err(|err| err.body)
			else {
				// Retry after 5 seconds, but break if the exit signal is received
				time::sleep(Duration::from_secs(5))
					.with_cancel_check()
					.await?;
				continue 'main;
			};
			info!("Connected to the server");

			let mut pinned_stream = pin!(stream);
			// Intentionally set to zero so that we sync immediately upon start
			let mut pinned_sleeper = Box::pin(time::sleep(Duration::from_secs(0)));

			let Ok(()) = pinned_stream
				.send(
					StreamRunnerDataForWorkspaceClientMsg::SetRunnerExposureType {
						exposure_type: E::runner_exposure_type(&self.state.config),
					},
				)
				.await
			else {
				// Retry after 5 seconds, but break if the exit signal is received
				time::sleep(Duration::from_secs(5))
					.with_cancel_check()
					.await?;
				continue 'main;
			};

			trace!("Syncing all resources before starting streaming");
			'message: loop {
				let Some(stream_message) = future::select(
					&mut pinned_sleeper,
					future::select(pinned_stream.next(), Box::pin(receiver.recv())),
				)
				.with_cancel_check()
				.await?
				.into_right() else {
					// Every 2 hours, resync all resources to make sure everything is fine
					info!("Resyncing all resources with upstream to make sure everything is fine");
					while let Err(err) = self
						.resync_all_resources_with_upstream(
							workspace_id,
							runner_id,
							&api_token,
							&user_agent,
						)
						.await
					{
						error!("Failed to sync all resources: {:?}", err);
						error!("Retrying in 1 second");
						// Retry after 1 seconds, but break if the exit signal is received
						time::sleep(Duration::from_secs(1))
							.with_cancel_check()
							.await?;
					}
					info!("All resources synced successfully");

					pinned_sleeper = Box::pin(time::sleep(
						if cfg!(debug_assertions) {
							Duration::from_secs(30) // 30 seconds in debug mode
						} else {
							Duration::from_hours(2)
						},
					));

					continue 'message;
				};

				let reconcile_message = match stream_message {
					Either::Left((reconcile_message, _)) => reconcile_message,
					Either::Right((executor_message, _)) => {
						let Some(executor_message) = executor_message else {
							// The executor message channel has been closed. This should not happen
							error!("Executor message channel has been closed");
							continue 'message;
						};
						let client_msg = match executor_message {
							ExecutorStatusUpdate::DeploymentStatusUpdated {
								deployment_id,
								status,
							} => StreamRunnerDataForWorkspaceClientMsg::DeploymentStatusUpdated {
								id: deployment_id,
								status,
							},
						};
						let Ok(()) = pinned_stream.send(client_msg).await.inspect_err(|err| {
							error!("Failed to send client message: {:?}", err);
							error!("Retrying connection to server");
						}) else {
							// Retry after 5 seconds, but break if the exit signal is received
							time::sleep(Duration::from_secs(5))
								.with_cancel_check()
								.await?;
							continue 'main;
						};

						continue 'message;
					}
				};

				match reconcile_message {
					Some(Ok(response)) => {
						let mut try_count = 0;
						while let Err(err) = self.handle_server_message(response.clone()).await {
							// Failed to handle the message. Retry after 1 second
							error!("Failed to handle the message: {err}");
							warn!("Retrying in 1 second...");
							time::sleep(Duration::from_secs(1))
								.with_cancel_check()
								.await?;
							try_count += 1;

							if try_count >= 5 {
								error!("Handing server message failed more than 5 times.");
								error!("Restarting connection to server");
								continue 'main;
							}
						}
					}
					Some(Err(err)) => {
						// Data from the websocket failed
						error!("Failed to connect to the server: {:?}", err);
						error!("Retrying in 1 second");

						// Retry after 1 second, but break if the exit signal is received
						time::sleep(Duration::from_secs(1))
							.with_cancel_check()
							.await?;

						break 'message;
					}
					None => {
						// Websocket disconnected. Reconnect
						error!("Connection to server closed");
						error!("Retrying in 2 seconds");
						// Retry after 2 seconds, but break if the exit signal is received
						time::sleep(Duration::from_secs(2))
							.with_cancel_check()
							.await?;

						break 'message;
					}
				}
			}
		}
	}

	/// Handle a message from the server. This function will handle the message
	/// from the server and run the reconciliation for the resource that the
	/// message is for.
	#[instrument(skip(self))]
	async fn handle_server_message(
		&self,
		msg: StreamRunnerDataForWorkspaceServerMsg,
	) -> Result<(), RunnerError> {
		use StreamRunnerDataForWorkspaceServerMsg::*;

		let mut transaction = self.state.database.begin().await?;

		// Extract the deployment ID before the match moves the data.
		// Uuid is Copy so this is free.
		let deployment_id = match &msg {
			DeploymentCreated { deployment, .. } | DeploymentUpdated { deployment, .. } => {
				Some(deployment.id)
			}
			DeploymentDeleted { id } => Some(*id),
			ExposureTypeRequired => None,
		};

		let is_delete = matches!(msg, DeploymentDeleted { .. });

		match msg {
			DeploymentCreated {
				deployment,
				running_details,
			} => {
				self.create_deployment_in_database(&mut transaction, deployment, running_details)
					.await?;
			}
			DeploymentUpdated {
				deployment,
				running_details,
			} => {
				self.delete_deployment_in_database(&mut transaction, deployment.id)
					.await?;
				self.create_deployment_in_database(&mut transaction, deployment, running_details)
					.await?;
			}
			DeploymentDeleted { id } => {
				self.delete_deployment_in_database(&mut transaction, id)
					.await?;
			}
			ExposureTypeRequired => {
				warn!("Server requested exposure type to be set again");
			}
		}

		transaction.commit().await?;

		// Directly notify the task executor now that data is committed.
		// This bypasses the SQLite update hook (which fires with unstable rowids)
		// for the managed mode path.
		if let Some(deployment_id) = deployment_id {
			if is_delete {
				self.delete_running_deployment(deployment_id).await;
			} else {
				self.upsert_running_deployment(deployment_id).await;
			}
		}

		Ok(())
	}
}
