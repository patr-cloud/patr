use std::str::FromStr;

use apalis::prelude::*;
use apalis_cron::CronStream;
use apalis_postgres::{Config, PostgresStorage, shared::SharedPostgresStorage};
use cron::Schedule;
use futures::FutureExt;
use serde::{Deserialize, Serialize};

use crate::prelude::*;

/// The background workers for rendering, and sending emails.
pub mod mailer;

/// The cron job that re-verifies verified domains every 6 hours.
mod reverify_verified_domains;
/// The cron job that verifies unverified domains every 2 hours.
mod verify_unverified_domains;

use self::{reverify_verified_domains::*, verify_unverified_domains::*};

/// The type of background task to be performed by the worker. This is used to
/// differentiate between different types of tasks, such as sending emails or
/// performing database maintenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "camelCase")]
pub enum WorkerTaskType {
	/// The task to send an email. This task is used to send an email using the
	/// worker, and contains the necessary information to send the email, such
	/// as the recipient and the type of email to be sent.
	Email(Email),
}

/// Sets up the storage backend for the worker. This storage backend is where
/// the jobs get stored (and can be used to queue jobs as well).
pub fn setup(pool: &sqlx::Pool<DatabaseType>) -> PostgresStorage<WorkerTaskType> {
	PostgresStorage::new(pool)
}

/// Initializes the worker by running any necessary setup tasks, such as
/// database migrations.
pub async fn initialize(state: &AppState) -> Result<(), sqlx::Error> {
	PostgresStorage::setup(&state.database).await
}

/// Sets up the monitor for the worker. This monitor is used to monitor the
/// worker and its jobs.
pub async fn run(state: &AppState) {
	Monitor::new()
		.register({
			let state = state.clone();
			move |_| {
				let backend = SharedPostgresStorage::new(state.database.clone())
					.make_shared()
					.expect("Failed to create shared postgres storage for worker");

				WorkerBuilder::new("verify-unverified-domains")
					.backend(
						CronStream::new(
							// Every 2 hours
							Schedule::from_str("0 */2 * * * *").expect(
								"Failed to parse cron schedule for verify-unverified-domains",
							),
						)
						.pipe_to(backend),
					)
					.data(state.clone())
					.build(verify_unverified_domains)
			}
		})
		.register({
			let state = state.clone();
			move |_| {
				let backend = SharedPostgresStorage::new(state.database.clone())
					.make_shared()
					.expect("Failed to create shared postgres storage for worker");

				WorkerBuilder::new("reverify-verified-domains")
					.backend(
						CronStream::new(
							// Every 6 hours
							Schedule::from_str("0 */6 * * * *").expect(
								"Failed to parse cron schedule for reverify-verified-domains",
							),
						)
						.pipe_to(backend),
					)
					.data(state.clone())
					.build(reverify_verified_domains)
			}
		})
		// TODO worker to clean up users who have signed up but haven't verified their email
		// TODO worker to clean up password reset tokens that have expired
		.register({
			let state = state.clone();
			move |_| {
				let backend = PostgresStorage::new_with_notify(&state.database, &Config::default());

				WorkerBuilder::new("background-worker")
					.backend(backend)
					.data(state.clone())
					.build(
						async |task: WorkerTaskType, state: Data<AppState>| match task {
							WorkerTaskType::Email(email) => mailer::send_emails(email, state).await,
						},
					)
			}
		})
		.run_with_signal(super::exit_signal().then(async |_| Ok(())))
		.await
		.unwrap();
}
