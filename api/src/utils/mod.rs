mod eve_context;
mod eve_middlewares;

pub mod constants;
pub mod logger;
pub mod settings;
pub mod validator;

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chrono::Utc;
pub use eve_context::*;
pub use eve_middlewares::*;
use eve_rs::Error as EveError;

pub type ErrorData = ();
pub type Error = EveError<ErrorData>;

pub fn get_current_time_millis() -> u64 {
	Utc::now().timestamp_millis() as u64
}
