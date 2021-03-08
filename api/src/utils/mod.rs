mod eve_context;
mod eve_middlewares;

pub mod constants;
pub mod logger;
#[allow(unused_variables)]
pub mod mailer;
pub mod settings;
pub mod sms;
pub mod validator;

pub use eve_context::*;
pub use eve_middlewares::*;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub fn get_current_time() -> Duration {
	SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.expect("Time went backwards. Wtf?")
}

pub fn get_current_time_millis() -> u64 {
	get_current_time().as_millis() as u64
}

#[cfg(not(feature = "sample-data"))]
pub fn generate_new_otp() -> String {
	use rand::Rng;

	let otp: u32 = rand::thread_rng().gen_range(0, 1_000_000);

	if otp < 10 {
		format!("00000{}", otp)
	} else if otp < 100 {
		format!("0000{}", otp)
	} else if otp < 1000 {
		format!("000{}", otp)
	} else if otp < 10000 {
		format!("00{}", otp)
	} else if otp < 100000 {
		format!("0{}", otp)
	} else {
		format!("{}", otp)
	}
}

#[cfg(feature = "sample-data")]
pub fn generate_new_otp() -> String {
	format!("000000")
}
