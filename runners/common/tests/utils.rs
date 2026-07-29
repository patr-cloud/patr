use std::time::{Duration, Instant};

use common::prelude::{DatabaseType, Row, Uuid, query};
use headers::UserAgent;

/// The User-Agent header value used for all test API calls. Mirrors the
/// pattern in `api/tests/utils.rs`.
pub const TEST_USER_AGENT: UserAgent = UserAgent::from_static(concat!(
	"cargo-test/",
	env!("CARGO_PKG_VERSION_MAJOR"),
	".",
	env!("CARGO_PKG_VERSION_MINOR"),
	".",
	env!("CARGO_PKG_VERSION_PATCH"),
));

/// Polls `check` every 50ms until it returns true or `timeout` elapses.
/// Panics if the timeout is reached. Same pattern ractor uses internally
/// (`common_test::periodic_check`), but theirs is `#[cfg(test)] pub(crate)`
/// so we can't reuse it.
pub async fn periodic_check<F: Fn() -> bool>(check: F, timeout: Duration) {
	let start = Instant::now();
	while start.elapsed() < timeout {
		if check() {
			return;
		}
		tokio::time::sleep(Duration::from_millis(50)).await;
	}
	assert!(check(), "periodic check timed out after {timeout:?}");
}

/// Polls the `deployment` table every 50ms until `id`'s status equals
/// `expected`, or `timeout` elapses. Panics with the status it last saw.
///
/// Use this rather than waiting on a `GetStatus` mock call and then reading the
/// row: the mock records the call as the executor is entered, but the status
/// UPDATE lands afterwards in `handle_status_reconciliation`, so the call is
/// always observable before the write it stands in for.
pub async fn wait_for_deployment_status(
	database: &sqlx::Pool<DatabaseType>,
	id: Uuid,
	expected: &str,
	timeout: Duration,
) {
	let start = Instant::now();
	let mut last = None::<String>;

	while start.elapsed() < timeout {
		last = Some(
			query("SELECT status FROM deployment WHERE id = $1")
				.bind(id)
				.fetch_one(database)
				.await
				.expect("failed to read deployment status")
				.get("status"),
		);

		if last.as_deref() == Some(expected) {
			return;
		}

		tokio::time::sleep(Duration::from_millis(50)).await;
	}

	panic!("deployment status was {last:?} after {timeout:?}, expected {expected:?}");
}
