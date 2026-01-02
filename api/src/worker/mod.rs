use std::str::FromStr;

use apalis::prelude::*;
use apalis_codec::json::JsonCodec;
use apalis_cron::{CronStream, Tick};
use apalis_postgres::{Config, PgNotify, PostgresStorage};
use cron::Schedule;
use futures::FutureExt;

use crate::prelude::*;

/// The cron job that re-verifies verified domains every 6 hours.
mod reverify_verified_domains;
/// The cron job that verifies unverified domains every 2 hours.
mod verify_unverified_domains;

use self::{reverify_verified_domains::*, verify_unverified_domains::*};

/// Sets up the storage backend for the worker. This storage backend is where
/// the jobs get stored (and can be used to queue jobs as well).
pub fn setup(
	pool: &sqlx::Pool<DatabaseType>,
) -> PostgresStorage<Tick, Vec<u8>, JsonCodec<Vec<u8>>, PgNotify> {
	PostgresStorage::new_with_notify(pool, &Config::default())
}

/// Sets up the monitor for the worker. This monitor is used to monitor the
/// worker and its jobs.
pub async fn run(state: &AppState) {
	Monitor::new()
		.register({
			let state = state.clone();
			move |_| {
				WorkerBuilder::new("verify-unverified-domains")
					.backend(
						CronStream::new(
							// Every 2 hours
							Schedule::from_str("0 */2 * * *").expect(
								"Failed to parse cron schedule for verify-unverified-domains",
							),
						)
						.pipe_to(state.worker.clone()),
					)
					.data(state.clone())
					.build(verify_unverified_domains)
			}
		})
		.register({
			let state = state.clone();
			move |_| {
				WorkerBuilder::new("reverify-verified-domains")
					.backend(
						CronStream::new(
							// Every 6 hours
							Schedule::from_str("0 */6 * * *").expect(
								"Failed to parse cron schedule for reverify-verified-domains",
							),
						)
						.pipe_to(state.worker.clone()),
					)
					.data(state.clone())
					.build(reverify_verified_domains)
			}
		})
		.run_with_signal(super::exit_signal().then(async |_| Ok(())))
		.await
		.unwrap();
}
