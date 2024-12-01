use std::pin::pin;

use futures::future::Either;
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
		.listen(constants::DATABASE_CHANNEL)
		.await
		.expect("unable to listen to the notification channel");

	let mut exit_signal = pin!(crate::exit_signal());

	loop {
		let Either::Right((message, _)) =
			futures::future::select(&mut exit_signal, pin!(listener.recv())).await
		else {
			// Left branch is the exit signal
			info!("Received SIGINT, shutting down");
			break;
		};

		if let Ok(message) = message {
			_ = state
				.redis
				.publish(message.channel(), message.payload())
				.await;
		}
	}
}
