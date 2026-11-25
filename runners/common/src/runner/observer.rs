use std::collections::{BTreeMap, BTreeSet};

use futures::{
	StreamExt,
	future::{self, Either},
};
use models::api::workspace::deployment::*;
use tokio::{
	sync::mpsc,
	time::{self, Duration, Instant},
};

use super::{controller::ControllerMessage, websocket::WebSocketMessage};
use crate::prelude::*;

/// How often the observer takes a status snapshot without a hint.
const OBSERVE_INTERVAL: Duration = Duration::from_secs(5);

/// How long the observer waits after a hint before taking a snapshot, so a
/// burst of backend events is observed once.
const HINT_DELAY: Duration = Duration::from_secs(1);

/// Messages for the observer.
pub enum ObserverMessage {
	/// The outcome of a deployment's last reconcile. A deployment whose
	/// objects failed to apply is `Errored`, whatever is still running.
	ApplyOutcome {
		/// The deployment that was reconciled.
		deployment_id: Uuid,
		/// Whether its objects failed to apply.
		failed: bool,
	},
}

/// Works out each deployment's observed status from the backend and the
/// controller's last reconcile until the runner exits, records it in
/// `deployment.observed_status`, and tells the upstream to publish it.
pub async fn run<E: RunnerExecutor>(
	mut receiver: mpsc::UnboundedReceiver<ObserverMessage>,
	controller_tx: mpsc::UnboundedSender<ControllerMessage>,
	websocket_tx: mpsc::UnboundedSender<WebSocketMessage>,
	executor: E,
	database: sqlx::Pool<DatabaseType>,
) -> Result<!, RunnerError> {
	let mut watch = Box::pin(executor.watch());
	// The deployments whose last reconcile failed to apply.
	let mut apply_failed = BTreeSet::new();

	// This is set to zero intentionally so that the first snapshot is taken
	// immediately, and then every observe interval after that.
	let mut sleep_future = Box::pin(time::sleep(Duration::from_secs(0)));

	loop {
		let observer_future = future::select(
			sleep_future,
			future::select(Box::pin(receiver.recv()), watch.next()),
		)
		.with_cancel_check()
		.await?;
		match observer_future {
			Either::Left(((), _)) => {
				sleep_future = Box::pin(time::sleep(OBSERVE_INTERVAL));
			}
			Either::Right((Either::Left((message, _)), pending_sleep)) => {
				sleep_future = pending_sleep;
				let Some(message) = message else {
					// The observer message channel has been closed. This should not happen
					error!("Observer message channel has been closed");
					continue;
				};

				match message {
					ObserverMessage::ApplyOutcome {
						deployment_id,
						failed,
					} => {
						if failed {
							apply_failed.insert(deployment_id);
						} else {
							apply_failed.remove(&deployment_id);
						}
					}
				}
			}
			Either::Right((Either::Right((owner, _)), mut pending_sleep)) => {
				let Some(owner) = owner else {
					// The backend's event stream ended. Watch it again
					warn!("The backend's event stream ended, watching it again in 1 second");
					// Retry after 1 second, but break if the exit signal is received
					time::sleep(Duration::from_secs(1))
						.with_cancel_check()
						.await?;
					watch = Box::pin(executor.watch());
					sleep_future = pending_sleep;
					continue;
				};

				let _ = controller_tx.send(ControllerMessage::Enqueue(BTreeSet::from([owner])));

				// Take the next snapshot shortly after the hint, unless one is due
				// sooner
				let hint_deadline = Instant::now() + HINT_DELAY;
				if pending_sleep.deadline() > hint_deadline {
					pending_sleep.as_mut().reset(hint_deadline);
				}
				sleep_future = pending_sleep;
				continue;
			}
		}

		let observed = match executor.observe().with_cancel_check().await? {
			Ok(observed) => observed,
			Err(err) => {
				warn!(?err, "Failed to observe deployments");
				continue;
			}
		};

		let current = match query!(
			r#"
			SELECT
				id,
				observed_status
			FROM
				deployment;
			"#
		)
		.fetch_all(&database)
		.await
		{
			Ok(rows) => rows
				.into_iter()
				.map(|row| (row.id, row.observed_status))
				.collect::<BTreeMap<_, _>>(),
			Err(err) => {
				error!(?err, "Failed to load the recorded observed statuses");
				continue;
			}
		};

		apply_failed.retain(|deployment_id| current.contains_key(deployment_id));

		for (deployment_id, current) in current {
			let status = if apply_failed.contains(&deployment_id) {
				ObservedDeploymentStatus::Errored
			} else {
				observed
					.get(&deployment_id)
					.copied()
					.unwrap_or(ObservedDeploymentStatus::Stopped)
			};
			if current == Some(status) {
				continue;
			}

			if let Err(err) = query!(
				r#"
				UPDATE
					deployment
				SET
					observed_status = $1
				WHERE
					id = $2;
				"#,
				status,
				deployment_id
			)
			.execute(&database)
			.await
			{
				error!(%deployment_id, ?err, "Failed to record the observed status");
				continue;
			}

			let _ = websocket_tx.send(WebSocketMessage::SendStatusUpdate {
				resource_id: deployment_id,
				status,
			});
		}
	}
}
