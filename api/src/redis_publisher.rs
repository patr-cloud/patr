use rustis::commands::PubSubCommands;
use sqlx::postgres::PgListener;

use crate::prelude::*;

/// Runs a background task that listens to the database for notifications and
/// publishes them to Redis. Any websocket connections that want to listen in on
/// changes to the database can then subscribe to the Redis channel and receive
/// the notifications.
#[instrument(skip(state))]
pub async fn run(state: &AppState) {
	let mut listener = PgListener::connect_with(&state.database)
		.await
		.expect("unable to connect to database");

	listener
		.listen_all(["channel1", "channel2", "channel3"])
		.await
		.expect("unable to listen to channels");

	while let Ok(message) = listener.recv().await {
		println!("Received notification: {:?}", message);
		_ = state
			.redis
			.publish(message.channel(), message.payload())
			.await;
	}
}
