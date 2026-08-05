use std::str::FromStr;

use apalis::prelude::*;
use apalis_cron::CronStream;
use apalis_postgres::{PostgresStorage, shared::SharedPostgresStorage};
use cron::Schedule;
use futures::FutureExt;
use serde::{Deserialize, Serialize};

use crate::prelude::*;

/// The background workers for rendering, and sending emails.
pub mod mailer;

/// The cron job that deletes workspace invites that have been expired for
/// longer than the retention window. Public so the integration tests can drive
/// it directly.
pub mod cleanup_expired_invites;

use self::cleanup_expired_invites::cleanup_expired_invites;

cfg_if! {
	if #[cfg(feature = "cloud")] {
		/// The cron job that cleans up managed URLs whose FQDN has been
		/// inactive for more than 7 days.
		mod cleanup_inactive_managed_urls;
		/// The cron job that cleans up domains that have been unverified for
		/// more than 7 days, including their managed URLs and custom
		/// hostnames.
		mod cleanup_unverified_domains;
		/// The cron job that re-verifies verified domains every 6 hours.
		mod reverify_verified_domains;
		/// The cron job that verifies managed URL active status every 2 hours
		/// and reconciles missing custom hostnames.
		mod verify_managed_url_active;
		/// The cron job that verifies unverified domains every 2 hours.
		mod verify_unverified_domains;

		use self::{
			cleanup_inactive_managed_urls::*,
			cleanup_unverified_domains::*,
			reverify_verified_domains::*,
			verify_managed_url_active::*,
			verify_unverified_domains::*,
		};
	}
}

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
	cfg_if! {
		if #[cfg(feature = "cloud")] {
			let monitor = Monitor::new()
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
				.register({
					let state = state.clone();
					move |_| {
						let backend = SharedPostgresStorage::new(state.database.clone())
							.make_shared()
							.expect("Failed to create shared postgres storage for worker");

						WorkerBuilder::new("verify-managed-url-active")
							.backend(
								CronStream::new(
									// Every 2 hours
									Schedule::from_str("0 */2 * * * *").expect(
										"Failed to parse cron schedule for verify-managed-url-active",
									),
								)
								.pipe_to(backend),
							)
							.data(state.clone())
							.build(verify_managed_url_active)
					}
				})
				.register({
					let state = state.clone();
					move |_| {
						let backend = SharedPostgresStorage::new(state.database.clone())
							.make_shared()
							.expect("Failed to create shared postgres storage for worker");

						WorkerBuilder::new("cleanup-unverified-domains")
							.backend(
								CronStream::new(
									// Every 6 hours
									Schedule::from_str("0 */6 * * * *").expect(
										"Failed to parse cron schedule for cleanup-unverified-domains",
									),
								)
								.pipe_to(backend),
							)
							.data(state.clone())
							.build(cleanup_unverified_domains)
					}
				})
				.register({
					let state = state.clone();
					move |_| {
						let backend = SharedPostgresStorage::new(state.database.clone())
							.make_shared()
							.expect("Failed to create shared postgres storage for worker");

						WorkerBuilder::new("cleanup-inactive-managed-urls")
							.backend(
								CronStream::new(
									// Every 6 hours
									Schedule::from_str("0 */6 * * * *").expect(
										"Failed to parse cron schedule for cleanup-inactive-managed-urls",
									),
								)
								.pipe_to(backend),
							)
							.data(state.clone())
							.build(cleanup_inactive_managed_urls)
					}
				});
		} else {
			let monitor = Monitor::new();
		}
	}

	// TODO worker to clean up users who have signed up but haven't verified their
	// email TODO worker to clean up password reset tokens that have expired
	monitor
		.register({
			// Registered outside the `cloud` gate, unlike the crons above —
			// those are all Cloudflare/domain jobs, but workspace invites exist
			// in both flavors.
			let state = state.clone();
			move |_| {
				let backend = SharedPostgresStorage::new(state.database.clone())
					.make_shared()
					.expect("Failed to create shared postgres storage for worker");

				WorkerBuilder::new("cleanup-expired-invites")
					.backend(
						CronStream::new(
							// Every day at 03:00
							Schedule::from_str("0 0 3 * * *").expect(
								"Failed to parse cron schedule for cleanup-expired-invites",
							),
						)
						.pipe_to(backend),
					)
					.data(state.clone())
					.build(cleanup_expired_invites)
			}
		})
		.register({
			let state = state.clone();
			move |_| {
				let backend = PostgresStorage::new(&state.database);

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
