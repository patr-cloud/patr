mod eve_context;
mod eve_middlewares;

pub mod constants;
pub mod logger;
pub mod settings;
pub mod validator;

pub use eve_context::*;
pub use eve_middlewares::*;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn get_current_time() -> u64 {
	SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.expect("Time went backwards. Wtf?")
		.as_millis() as u64
}
