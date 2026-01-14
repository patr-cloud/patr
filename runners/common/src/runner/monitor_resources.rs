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
		let (sender, mut receiver) = mpsc::unbounded_channel::<SqliteUpdateHook>();

		info!("Installing SQLite hook for updates");
		let update_sender = sender.clone();
		self.state
			.database
			.acquire()
			.await?
			.lock_handle()
			.await?
			.set_update_hook(move |params| {
				_ = update_sender.send(params.into());
			});

		let full_sync_interval = if cfg!(debug_assertions) {
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
					sleep_future = Box::pin(time::sleep(full_sync_interval));
				}
				Either::Right((update, next_sleep)) => {
					sleep_future = next_sleep;

					let Some(update) = update else {
						continue;
					};

					if !update.table.ends_with("_update_log") || update.database != "main" {
						continue;
					}
					if update.operation == SqliteOperation::Delete {
						continue;
					}
					if let SqliteOperation::Unknown(op_id) = update.operation {
						warn!("Ignoring unsupported SQLite operation: {op_id}");
						continue;
					}

					info!("Database update received: {:?}", update);

					let Ok(deployment_id) = query(
						r#"
                        SELECT
                            deployment_id,
                            update_type
                        FROM
                            deployment_update_log
                        WHERE
                            rowid = $1;
                        "#,
					)
					.bind(update.row_id)
					.fetch_optional(&self.state.database)
					.await
					.inspect_err(|err| {
						error!("Failed to fetch deployment update log: {}", err);
					}) else {
						_ = sender
							.send(update)
							.inspect_err(|err| error!("Unable to resend database update: {}", err));
						continue;
					};

					let Some((deployment_id, update_type)) = deployment_id.map(|row| {
						(
							row.get::<Uuid, _>("deployment_id"),
							row.get::<String, _>("update_type"),
						)
					}) else {
						// Deployment not found. It was probably deleted or the row was updated
						// subsequently with some other value.
						debug!(
							"Deployment for updated row `{}` not found in database. Skipping...",
							update.row_id
						);
						continue;
					};

					match update_type.as_str() {
						"insert" | "update" => {
							self.upsert_running_deployment(deployment_id).await;
						}
						"delete" => {
							self.delete_running_deployment(deployment_id).await;
						}
						unknown => {
							warn!("Unknown SQLite operation: {}", unknown);
						}
					}
				}
			}
		}
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
