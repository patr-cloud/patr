use std::pin::pin;

use futures::future::{self, Either};
use sqlx::sqlite::SqliteOperation;
use tokio::{
	sync::mpsc,
	time::{self, Duration},
};

use crate::{prelude::*, utils::SqliteUpdateHook};

impl<E> super::Runner<E>
where
	E: RunnerExecutor + Send + Sync + 'static,
{
	/// Monitor the resources and make sure that they are running. This function
	/// will listen for changes in the resources and make sure that they are
	/// running. The job of this function is to make sure that whatever is
	/// running is exactly as per what's in the database.
	#[instrument(skip(self))]
	pub(super) async fn monitor_resources(&self) -> Result<!, RunnerError> {
		let mut receiver = self.install_update_hook().await?;

		const FULL_SYNC_INTERVAL: Duration = if cfg!(debug_assertions) {
			Duration::from_secs(10)
		} else {
			Duration::from_secs(60 * 10) // 10 minutes
		};

		// This is set to zero intentionally so that during the first iteration
		// of the loop, we don't wait for the full sync interval. The first sync should
		// happen immediately and then after that we start waiting for the full sync
		// interval.
		let mut sleep_future = Box::pin(time::sleep(Duration::from_secs(0)));

		// Remember: The point of this loop is not to update the database or the
		// resource. Our job is simple: Make sure that for every resource in the
		// database, there is a task running. It's the task's job to update the
		// resource. As long as it's running, we are happy. So NO updating the
		// resource here whatsoever. All that happens in the task.
		loop {
			// Check if the update hook channel is still alive. If the pool
			// recycled the connection holding the hook, the sender is dropped
			// and recv() would return None immediately — causing a busy spin.
			if receiver.is_closed() {
				warn!(
					"SQLite update hook channel closed (pool connection recycled). \
					 Re-installing hook. Periodic reconciliation continues to run \
					 regardless — no updates will be missed."
				);
				match self.install_update_hook().await {
					Ok(new_receiver) => {
						info!("SQLite update hook re-installed successfully");
						receiver = new_receiver;
					}
					Err(err) => {
						error!(
							"Failed to re-install SQLite update hook: {}. \
							 Will retry in 1 second. Periodic reconciliation \
							 is still active.",
							err
						);
						time::sleep(Duration::from_secs(1))
							.with_cancel_check()
							.await?;
					}
				}
			}

			let receive_future = pin!(receiver.recv());
			let monitor_future = future::select(sleep_future, receive_future)
				.with_cancel_check()
				.await?;
			match monitor_future {
				Either::Left(((), _)) => {
					// Regularly (every 10 minutes in prod and 10 seconds in dev) reconcile all the
					// deployments. Check all resources in the local database and make sure they are
					// running on the runner.
					let Ok(()) = self.reconcile_resources().await else {
						time::sleep(Duration::from_secs(1))
							.with_cancel_check()
							.await?;
						sleep_future = Box::pin(time::sleep(Duration::from_millis(0)));
						continue;
					};

					_ = query(
						r#"
						DELETE FROM
							deployment_update_log
						WHERE
							deleted_at IS NOT NULL AND
							deleted_at < DATETIME('now', '-1 day');
						"#,
					)
					.execute(&self.state.database)
					.await
					.inspect_err(|err| {
						error!("Failed to clean up deployment update log: {}", err);
					});
					sleep_future = Box::pin(time::sleep(FULL_SYNC_INTERVAL));
				}
				Either::Right((update, pending_sleep)) => {
					let Some(update) = update else {
						// Channel returned None — will be caught by is_closed()
						// check at the top of the next iteration.
						sleep_future = pending_sleep;
						continue;
					};

					if !update.table.ends_with("_update_log") || update.database != "main" {
						sleep_future = pending_sleep;
						continue;
					}
					if update.operation == SqliteOperation::Delete {
						sleep_future = pending_sleep;
						continue;
					}
					if let SqliteOperation::Unknown(op_id) = update.operation {
						warn!("Ignoring unsupported SQLite operation: {op_id}");
						sleep_future = pending_sleep;
						continue;
					}

					trace!("Database update received: {:?}", update);

					// Don't look up by rowid — rowids are unstable (INSERT OR
					// REPLACE invalidates them). In managed mode,
					// handle_server_message notifies executors directly after
					// commit. For all other paths, the periodic reconciliation
					// picks up changes.
					sleep_future = pending_sleep;
				}
			}
		}
	}

	/// Install the SQLite update hook on a pool connection and return the
	/// receiver end of the channel. If the pool later recycles the connection,
	/// the sender is dropped, the channel closes, and the caller should
	/// re-invoke this to get a fresh hook.
	async fn install_update_hook(
		&self,
	) -> Result<mpsc::UnboundedReceiver<SqliteUpdateHook>, RunnerError> {
		let (sender, receiver) = mpsc::unbounded_channel::<SqliteUpdateHook>();

		info!("Installing SQLite update hook");
		self.state
			.database
			.acquire()
			.await?
			.lock_handle()
			.await?
			.set_update_hook(move |params| {
				_ = sender.send(params.into());
			});

		Ok(receiver)
	}

	/// Resync all the resources that the runner is responsible for. This
	/// function will sync the local database with the upstream API, making sure
	/// both are in sync.
	#[instrument(skip(self, api_token))]
	pub(super) async fn resync_all_resources_with_upstream(
		&self,
		workspace_id: Uuid,
		runner_id: Uuid,
		api_token: &BearerToken,
		user_agent: &UserAgent,
	) -> Result<(), RunnerError> {
		// Reconcile all resources
		self.resync_all_deployments_with_upstream(workspace_id, runner_id, api_token, user_agent)
			.await?;

		Ok(())
	}
}
