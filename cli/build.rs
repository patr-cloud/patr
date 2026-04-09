//! Forwards `PATR_BUILD_SHA` and sets `PATR_BUILD_DATE` at compile time
//! for use in `patr --version`.

use time::OffsetDateTime;

fn main() {
	if let Ok(sha) = std::env::var("PATR_BUILD_SHA") {
		println!("cargo:rustc-env=PATR_BUILD_SHA={sha}");
	}

	let now = OffsetDateTime::now_utc();
	println!(
		"cargo:rustc-env=PATR_BUILD_DATE={:04}-{:02}-{:02}",
		now.year(),
		now.month() as u8,
		now.day()
	);
}
