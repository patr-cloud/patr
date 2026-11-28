use std::{collections::BTreeSet, pin::pin};

use futures::{StreamExt, future::Either};
use models::rbac::ResourceType;
use rustis::commands::PubSubCommands;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgListener;
use tokio_util::sync::CancellationToken;

use crate::prelude::*;

/// What a runner should know about a deployment
mod deployment;
/// What a runner should know about a managed URL
mod managed_url;
/// What a runner should know about a secret
mod secret;

/// A resource a runner may be out of date about. It carries no verb: the
/// bridge loads the resource's current state and tells the runner what it
/// should know now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirtyResource {
	/// The workspace the runner and the resource belong to
	pub workspace_id: Uuid,
	/// The runner that may be out of date
	pub runner_id: Uuid,
	/// The type of the resource that changed
	pub resource_type: ResourceType,
	/// The resource that changed
	pub resource_id: Uuid,
}

/// Marks a resource dirty for a runner, so the bridge re-reads it and tells the
/// runner its current state. It's a `pg_notify`, so it only takes effect if the
/// transaction commits, and only once it has. It's lost if no bridge is
/// listening, which the runner's resync covers.
pub async fn mark_dirty(
	connection: &mut DatabaseConnection,
	dirty: DirtyResource,
) -> Result<(), ErrorType> {
	query!(
		r#"
		SELECT
			pg_notify($1, $2);
		"#,
		constants::DATABASE_CHANNEL,
		serde_json::to_string(&dirty)?,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Runs a background task that listens to the database for runner
/// notifications, loads the current state of each resource they name, and
/// publishes it to the runner's Redis channel, where its stream picks it up.
#[instrument(skip(state))]
pub async fn run(state: &AppState) {
	let mut listener = PgListener::connect_with(&state.database)
		.await
		.expect("unable to connect to database");

	listener
		.listen(constants::DATABASE_CHANNEL)
		.await
		.expect("unable to listen to the notification channel");

	let mut exit_signal = pin!(
		crate::GLOBAL_CANCEL_TOKEN
			.get_or_init(CancellationToken::new)
			.cancelled()
	);

	loop {
		let Either::Right((notification, _)) =
			futures::future::select(&mut exit_signal, pin!(listener.recv())).await
		else {
			// Left branch is the exit signal
			info!("Received SIGINT, quitting");
			break;
		};

		let Ok(notification) = notification
			.inspect_err(|err| error!("Failed to receive a database notification: {:?}", err))
		else {
			continue;
		};

		// Notifications that are already buffered are handled together, so
		// several changes to the same resource are published once.
		let notifications = std::iter::once(notification)
			.chain(std::iter::from_fn(|| listener.next_buffered()))
			.filter_map(|notification| {
				serde_json::from_str::<DirtyResource>(notification.payload())
					.inspect_err(|err| {
						error!("Failed to parse a dirty resource: {:?}", err);
					})
					.ok()
			})
			.collect::<BTreeSet<_>>();

		futures::stream::iter(notifications)
			.for_each_concurrent(4, |notification| async move {
				_ = publish_dirty_resource(state, notification)
					.await
					.inspect_err(|err| {
						error!(
							"Failed to publish {} `{}` to runner `{}`: {:?}",
							notification.resource_type,
							notification.resource_id,
							notification.runner_id,
							err
						);
					});
			})
			.await;
	}
	info!("Shutdown signal received, shutting down server gracefully");
}

/// Loads the current state of a dirty resource and publishes what the runner
/// should know about it now. Nothing is published to a runner that isn't
/// online, or that isn't in the resource's workspace.
async fn publish_dirty_resource(
	state: &AppState,
	DirtyResource {
		workspace_id,
		runner_id,
		resource_type,
		resource_id,
	}: DirtyResource,
) -> Result<(), ErrorType> {
	let mut connection = state.database.acquire().await?;

	let runner_online = query!(
		r#"
		SELECT
			IS_RUNNER_ONLINE(is_connected, last_seen) AS "online!"
		FROM
			runner
		WHERE
			id = $1 AND
			workspace_id = $2 AND
			deleted IS NULL;
		"#,
		runner_id,
		workspace_id,
	)
	.fetch_optional(&mut *connection)
	.await?
	.is_some_and(|runner| runner.online);

	if !runner_online {
		trace!("Runner `{runner_id}` isn't online, skipping {resource_type} `{resource_id}`");
		return Ok(());
	}

	let message = match resource_type {
		ResourceType::Deployment => {
			deployment::load_event(&mut connection, workspace_id, runner_id, resource_id).await?
		}
		ResourceType::ManagedURL => {
			managed_url::load_event(&mut connection, workspace_id, runner_id, resource_id).await?
		}
		ResourceType::Secret => {
			let Some(message) =
				secret::load_event(&mut connection, workspace_id, resource_id).await?
			else {
				trace!("Secret `{resource_id}` doesn't exist anymore, skipping");
				return Ok(());
			};
			message
		}
		_ => {
			warn!("Runners aren't told about {resource_type} `{resource_id}`, skipping");
			return Ok(());
		}
	};

	state
		.redis
		.publish(
			redis::keys::runner_events(&workspace_id, &runner_id),
			serde_json::to_string(&message)?,
		)
		.await?;

	Ok(())
}
