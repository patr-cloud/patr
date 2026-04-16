use std::time::{Duration, Instant};

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
