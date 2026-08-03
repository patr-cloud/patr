//! Forwards `PATR_BUILD_SHA` + `PATR_BUILD_CHANNEL` + `PATR_BUILD_VERSION` and
//! sets `PATR_BUILD_DATE` at compile time for use in `patr --version` and
//! `patr upgrade`.
//!
//! `PATR_TEST_API_BASE_URL` is forwarded the same way, but is a test-only hook
//! (see `utils::constants::API_BASE_URL`) — CI never sets it for release
//! builds.

use time::OffsetDateTime;

fn main() {
	for key in [
		"PATR_BUILD_SHA",
		"PATR_BUILD_CHANNEL",
		"PATR_BUILD_VERSION",
		"PATR_TEST_API_BASE_URL",
	] {
		println!("cargo:rerun-if-env-changed={key}");
		if let Ok(value) = std::env::var(key) {
			println!("cargo:rustc-env={key}={value}");
		}
	}

	let now = OffsetDateTime::now_utc();
	println!(
		"cargo:rustc-env=PATR_BUILD_DATE={:04}-{:02}-{:02}",
		now.year(),
		now.month() as u8,
		now.day()
	);
}
