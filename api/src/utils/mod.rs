mod eve_context;
mod eve_middlewares;

pub mod constants;
pub mod errors;
pub mod logger;
pub mod settings;
pub mod validator;

use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub use eve_context::*;
pub use eve_middlewares::*;
use eve_rs::Error as EveError;

pub type ErrorData = errors::APIError;
pub type Error = EveError<ErrorData>;

pub fn get_current_time() -> Duration {
	SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.expect("Time went backwards. Wtf?")
}

pub fn get_current_time_millis() -> u64 {
	get_current_time().as_millis() as u64
}
