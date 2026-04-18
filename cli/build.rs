//! Forwards `PATR_BUILD_SHA` + `PATR_BUILD_CHANNEL` + `PATR_BUILD_VERSION` and
//! sets `PATR_BUILD_DATE` at compile time for use in `patr --version` and
//! `patr upgrade`.

use time::OffsetDateTime;

fn main() {
	for key in ["PATR_BUILD_SHA", "PATR_BUILD_CHANNEL", "PATR_BUILD_VERSION"] {
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
